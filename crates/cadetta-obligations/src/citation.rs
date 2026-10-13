//! Citation digests and source acknowledgement (`cadetta ack ots:PATH --role source`).
//!
//! Every `cite` item an `ots` reaches is acknowledged by an `attest citation:<cite path>` record
//! whose digest is the item's `Citation.digest` (type, fields and the cited file's digest, or the
//! `absent` marker). Notes and pseudo-citations are not items and have no record. A later field
//! or file change makes the record stale, which is S004 alone, never O001.
//! Specification: `docs/spec/obligations.md` 2.3 and 3, `docs/spec/lints.md` S004.
//! frob:ticket 01M4VB63M63PQVRKN0HDXQP567
//! frob:ticket 01M4ST8ET5NPMBSVXFVD4WN8XP

use cadetta_ir::model::Citation;
use tracing::debug;

use crate::graph::{ObligationId, ObligationKind, Verdict};
use crate::lock::Attest;
use crate::staleness::{AttestState, Signer, attest_state, record_attest};

/// The role under which a cited source is acknowledged.
pub const SOURCE_ROLE: &str = "source";

/// Outcome of checking one citation against its acknowledgement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CitationCheck {
    /// A cite item never acknowledged.
    Open,
    /// The acknowledged digest equals the current one.
    Current,
    /// The S004 fact: a field or the cited file changed since acknowledged.
    Changed,
}

impl CitationCheck {
    /// Whether the citation obligation is stale (S004 applies).
    pub fn is_stale(&self) -> bool {
        matches!(self, Self::Changed)
    }
}

/// The citation obligation id for a `cite` item's canonical path.
pub fn citation_id(path: &str) -> ObligationId {
    ObligationId::new(ObligationKind::Citation, path)
}

/// Acknowledge a cite item as `signer`, recording its `Citation.digest` in an `attest` record.
pub fn ack_citation(c: &Citation, signer: Signer) -> Result<Attest, String> {
    let id = citation_id(&c.path);
    debug!(%id, role = %signer.role, "citation acknowledged");
    record_attest(
        id,
        signer,
        crate::staleness::Judgement {
            verdict: Verdict::Holds,
            margin: "-".into(),
        },
        &c.digest,
        &[],
    )
}

/// Check a cite item against its acknowledgement (the S004 decision).
pub fn check_citation(c: &Citation, att: Option<&Attest>) -> CitationCheck {
    let out = match attest_state(att, &c.digest.0, &[]) {
        AttestState::Open => CitationCheck::Open,
        AttestState::Current => CitationCheck::Current,
        AttestState::Stale(_) => CitationCheck::Changed,
    };
    debug!(path = %c.path, ?out, "citation checked");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lock::Date;
    use cadetta_ir::digest::digest_bytes;
    use cadetta_ir::model::{CitationType, CiteField};
    use std::collections::BTreeMap;

    fn cite(ty: CitationType, title: &str, file: Option<(&str, &[u8])>) -> Citation {
        let mut f = BTreeMap::new();
        f.insert("title".to_owned(), CiteField::Text(title.into()));
        if let Some((p, _)) = file {
            f.insert("file".to_owned(), CiteField::Text(p.into()));
        }
        Citation::new("p::c", ty, f, file.map(|(_, b)| digest_bytes(b)))
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
    fn ack_records_citation_digest_under_cite_path() {
        let c = cite(CitationType::Datasheet, "T", Some(("cites/ds.pdf", b"v1")));
        let a = ack_citation(&c, signer()).unwrap();
        assert_eq!(a.id.to_string(), "citation:p::c");
        assert_eq!(a.role, "source");
        assert_eq!(a.digest.as_str(), c.digest.0);
        assert_eq!(check_citation(&c, Some(&a)), CitationCheck::Current);
        assert_eq!(check_citation(&c, None), CitationCheck::Open);
        // A url-only cite is an item too: it has a record (fields only, `absent` file).
        let url = cite(CitationType::Web, "T", None);
        let u = ack_citation(&url, signer()).unwrap();
        assert_eq!(check_citation(&url, Some(&u)), CitationCheck::Current);
    }

    // frob:tests crates/cadetta-obligations/src/citation.rs::check_citation kind=unit
    #[test]
    fn field_or_file_change_is_s004() {
        let c = cite(CitationType::Datasheet, "T", Some(("cites/ds.pdf", b"v1")));
        let a = ack_citation(&c, signer()).unwrap();
        let file_changed = cite(CitationType::Datasheet, "T", Some(("cites/ds.pdf", b"v2")));
        assert!(check_citation(&file_changed, Some(&a)).is_stale());
        let text_changed = cite(CitationType::Datasheet, "U", Some(("cites/ds.pdf", b"v1")));
        assert!(check_citation(&text_changed, Some(&a)).is_stale());
        assert!(!check_citation(&c, Some(&a)).is_stale());
        // A file that goes missing digests as `absent`, which is also a change.
        let gone = cite(CitationType::Datasheet, "T", None);
        assert!(check_citation(&gone, Some(&a)).is_stale());
    }
}
