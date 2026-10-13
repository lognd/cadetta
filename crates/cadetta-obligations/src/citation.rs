//! Citation digests and source acknowledgement (`cadetta ack ots:PATH --role source`).
//!
//! Only `file` citations are drift-detected: the acknowledged digest covers the cited file's
//! content and the citation text (target and locator). Literal, URL and other citations carry
//! no file digest and are never checked. Citation staleness is reported as S004 only.
//! Specification: `docs/spec/obligations.md` 2.3 and 3, `docs/spec/lints.md` S004.
//! frob:ticket 01M4ST8ET5NPMBSVXFVD4WN8XP

use cadetta_ir::digest::digest_bytes;
use cadetta_ir::model::{Citation, CitationKind, Digest};
use tracing::{debug, trace};

use crate::graph::{ObligationId, ObligationKind, Verdict};
use crate::lock::Attest;
use crate::staleness::{AttestState, Signer, attest_state, record_attest};

/// The role under which a cited source is acknowledged.
pub const SOURCE_ROLE: &str = "source";

/// Outcome of checking one citation against its acknowledgement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CitationCheck {
    /// No digest or reachability check applies (literal, URL, standard, pseudo-citations).
    NotChecked,
    /// A file citation never acknowledged.
    Open,
    /// The acknowledged digest equals the current one.
    Current,
    /// The S004 fact: the cited file or the citation text changed since acknowledged.
    Changed,
}

impl CitationCheck {
    /// Whether the citation obligation is stale (S004 applies).
    pub fn is_stale(&self) -> bool {
        matches!(self, Self::Changed)
    }
}

/// The citation obligation id for a cited file path.
pub fn citation_id(path: &str) -> ObligationId {
    ObligationId::new(ObligationKind::Citation, path)
}

/// Digest of a file citation: the cited file's digest plus the citation text.
///
/// `None` when the citation is not a `file` citation or the file could not be read.
pub fn citation_digest(c: &Citation) -> Option<Digest> {
    if c.kind != CitationKind::File {
        return None;
    }
    let file = c.file_digest.as_ref()?;
    let text = format!(
        "citation\n{}\n{}\n{}",
        c.target,
        c.locator.as_deref().unwrap_or(""),
        file.0
    );
    Some(digest_bytes(text.as_bytes()))
}

/// Acknowledge a file citation as `signer`, recording its digest in an `attest` record.
///
/// Errors when the citation is not a drift-detected file citation (nothing to acknowledge
/// by digest) or its file was unreadable.
pub fn ack_citation(c: &Citation, signer: Signer) -> Result<Attest, String> {
    let digest = citation_digest(c).ok_or_else(|| {
        format!(
            "citation `{}` is not a readable local file; only file citations are digested",
            c.name
        )
    })?;
    let id = citation_id(&c.target);
    debug!(%id, role = %signer.role, "citation acknowledged");
    record_attest(
        id,
        signer,
        crate::staleness::Judgement {
            verdict: Verdict::Holds,
            margin: "-".into(),
        },
        &digest,
        &[],
    )
}

/// Check a citation against its acknowledgement (the S004 decision).
pub fn check_citation(c: &Citation, att: Option<&Attest>) -> CitationCheck {
    let Some(current) = citation_digest(c) else {
        trace!(name = %c.name, kind = ?c.kind, "citation not digest-checked");
        return CitationCheck::NotChecked;
    };
    let out = match attest_state(att, &current.0, &[]) {
        AttestState::Open => CitationCheck::Open,
        AttestState::Current => CitationCheck::Current,
        AttestState::Stale(_) => CitationCheck::Changed,
    };
    debug!(name = %c.name, ?out, "citation checked");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lock::Date;

    fn cite(
        kind: CitationKind,
        target: &str,
        locator: Option<&str>,
        file: Option<&[u8]>,
    ) -> Citation {
        Citation {
            name: "c".into(),
            target: target.into(),
            locator: locator.map(str::to_owned),
            kind,
            file_digest: file.map(digest_bytes),
        }
    }

    fn signer() -> Signer {
        Signer {
            role: SOURCE_ROLE.into(),
            by: "a@b.c".into(),
            at: Date::new("2026-10-13").unwrap(),
            reason: "read".into(),
        }
    }

    // frob:tests crates/cadetta-obligations/src/citation.rs::ack_citation kind=unit
    #[test]
    fn ack_records_file_digest() {
        let c = cite(CitationKind::File, "cites/ds.pdf", Some("p.2"), Some(b"v1"));
        let a = ack_citation(&c, signer()).unwrap();
        assert_eq!(a.id.to_string(), "citation:cites/ds.pdf");
        assert_eq!(a.role, "source");
        assert_eq!(a.digest.as_str(), citation_digest(&c).unwrap().0);
        assert_eq!(check_citation(&c, Some(&a)), CitationCheck::Current);
        assert_eq!(check_citation(&c, None), CitationCheck::Open);
        let url = cite(CitationKind::Url, "https://x", None, None);
        assert!(ack_citation(&url, signer()).is_err());
    }

    // frob:tests crates/cadetta-obligations/src/citation.rs::check_citation kind=unit
    #[test]
    fn file_or_text_change_is_s004() {
        let c = cite(CitationKind::File, "cites/ds.pdf", Some("p.2"), Some(b"v1"));
        let a = ack_citation(&c, signer()).unwrap();
        let file_changed = cite(CitationKind::File, "cites/ds.pdf", Some("p.2"), Some(b"v2"));
        assert!(check_citation(&file_changed, Some(&a)).is_stale());
        let text_changed = cite(CitationKind::File, "cites/ds.pdf", Some("p.3"), Some(b"v1"));
        assert!(check_citation(&text_changed, Some(&a)).is_stale());
        assert!(!check_citation(&c, Some(&a)).is_stale());
    }

    // frob:tests crates/cadetta-obligations/src/citation.rs::check_citation kind=unit
    #[test]
    fn literal_citation_is_not_checked() {
        let lit = cite(CitationKind::Standard, "ASTM A276", None, None);
        assert_eq!(check_citation(&lit, None), CitationCheck::NotChecked);
        assert_eq!(citation_digest(&lit), None);
    }
}
