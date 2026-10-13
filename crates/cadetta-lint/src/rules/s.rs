//! Lints S001 `ots-without-citation` and S002 `citation-unreachable`.
//!
//! Both rules read fact tables the driver builds from the IR and the
//! filesystem, so the rules stay pure. S001 fires per `ots` value with no
//! citation on it or on its component; S002 fires per local-file citation whose
//! path does not exist. Literal and URL citations are never checked.
//!
//! Specification: `docs/spec/lints.md` (S), `docs/spec/language.md` 3.

// frob:ticket 01M4SPWR4SAWTESPSK2DP99M5S

use cadetta_ir::model::SpanRef;
use tracing::{debug, trace};

use crate::diag::{Diagnostic, Location, Severity};

/// The S001 rule code.
pub const CODE_S001: &str = "S001";

/// The S002 rule code.
pub const CODE_S002: &str = "S002";

/// Citation facts about one `ots` value.
#[derive(Clone, Debug)]
pub struct OtsValueFacts {
    /// Canonical path of the value.
    pub path: String,
    /// True when the value itself carries a citation.
    pub cited: bool,
    /// True when the owning `ots` component carries a citation.
    pub component_cited: bool,
    /// Source span of the value.
    pub span: Option<SpanRef>,
}

/// How a citation points at its source.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CitationKind {
    /// A path to a local file; the only kind S002 checks.
    LocalFile,
    /// A URL; never checked.
    Url,
    /// A free-text literal (standard, handbook page); never checked.
    Literal,
}

/// Facts about one citation, with the driver's filesystem answer.
#[derive(Clone, Debug)]
pub struct CitationFacts {
    /// Canonical path of the cited item.
    pub path: String,
    /// What kind of citation it is.
    pub kind: CitationKind,
    /// The cited target text (a file path for `LocalFile`).
    pub target: String,
    /// True when a local-file target exists on disk (ignored for other kinds).
    pub exists: bool,
    /// Source span of the citation.
    pub span: Option<SpanRef>,
}

/// Run S001 over `values`: `ots` values with no citation on them or their component.
pub fn check_s001(
    values: &[OtsValueFacts],
    locate: impl Fn(&SpanRef) -> Location,
) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for v in values {
        if v.cited || v.component_cited {
            trace!(path = %v.path, "S001: cited");
            continue;
        }
        debug!(path = %v.path, "S001: ots value without citation");
        let mut d = Diagnostic::new(
            CODE_S001,
            Severity::Warning,
            format!(
                "ots value `{}` has no citation on it or its component",
                v.path
            ),
        );
        d.path = Some(v.path.clone());
        d.location = v.span.as_ref().map(&locate);
        out.push(d);
    }
    out
}

/// Run S002 over `cites`: local-file citations whose path does not exist.
pub fn check_s002(
    cites: &[CitationFacts],
    locate: impl Fn(&SpanRef) -> Location,
) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for c in cites {
        if c.kind != CitationKind::LocalFile || c.exists {
            trace!(path = %c.path, kind = ?c.kind, "S002: not a missing local file");
            continue;
        }
        debug!(path = %c.path, target = %c.target, "S002: citation unreachable");
        let mut d = Diagnostic::new(
            CODE_S002,
            Severity::Info,
            format!(
                "citation file `{}` on `{}` does not exist",
                c.target, c.path
            ),
        );
        d.path = Some(c.path.clone());
        d.location = c.span.as_ref().map(&locate);
        out.push(d);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn val(cited: bool, component_cited: bool) -> OtsValueFacts {
        OtsValueFacts {
            path: "demo::Bolt.pitch".into(),
            cited,
            component_cited,
            span: None,
        }
    }

    fn cite(kind: CitationKind, exists: bool) -> CitationFacts {
        CitationFacts {
            path: "demo::Bolt".into(),
            kind,
            target: "docs/bolt.pdf".into(),
            exists,
            span: None,
        }
    }

    // frob:tests crates/cadetta-lint/src/rules/s.rs::check_s001
    #[test]
    fn s001_fires_when_uncited() {
        let d = check_s001(&[val(false, false)], |_| unreachable!());
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].code, "S001");
        assert_eq!(d[0].severity, Severity::Warning);
    }

    // frob:tests crates/cadetta-lint/src/rules/s.rs::check_s001
    #[test]
    fn s001_quiet_when_value_or_component_cited() {
        let v = [val(true, false), val(false, true)];
        assert!(check_s001(&v, |_| unreachable!()).is_empty());
    }

    // frob:tests crates/cadetta-lint/src/rules/s.rs::check_s002
    #[test]
    fn s002_fires_on_missing_local_file() {
        let d = check_s002(&[cite(CitationKind::LocalFile, false)], |_| unreachable!());
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].code, "S002");
        assert_eq!(d[0].severity, Severity::Info);
    }

    // frob:tests crates/cadetta-lint/src/rules/s.rs::check_s002
    #[test]
    fn s002_quiet_for_existing_url_and_literal() {
        let c = [
            cite(CitationKind::LocalFile, true),
            cite(CitationKind::Url, false),
            cite(CitationKind::Literal, false),
        ];
        assert!(check_s002(&c, |_| unreachable!()).is_empty());
    }
}
