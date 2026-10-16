//! Lints S001 `ots-without-citation` and S002 `citation-unreachable`.
//!
//! Both rules read fact tables the driver builds from the IR and the
//! filesystem, so the rules stay pure. S001 fires once per wholly uncited
//! `ots` component, else per uncited value of a partly cited one; S002 fires per local-file citation whose
//! path does not exist. Literal and URL citations are never checked.
//!
//! Specification: `docs/spec/lints.md` (S), `docs/spec/language.md` 3.

// frob:ticket 01M4SPWR4SAWTESPSK2DP99M5S
// frob:ticket 01M4VB63M63PQVRKN0HDXQP567

use cadetta_ir::model::{Citation, CitationType, SpanRef};
use tracing::{debug, trace};

use crate::diag::{Diagnostic, Location, Severity};

/// The S001 rule code.
pub const CODE_S001: &str = "S001";

/// The S002 rule code.
pub const CODE_S002: &str = "S002";

/// The S004 rule code.
pub const CODE_S004: &str = "S004";

/// The S008 rule code.
pub const CODE_S008: &str = "S008";

/// Citation facts about one `ots` value.
#[derive(Clone, Debug)]
pub struct OtsValueFacts {
    /// Canonical path of the value.
    pub path: String,
    /// Canonical path of the owning `ots` component.
    pub component: String,
    /// Source span of the owning `ots` component.
    pub component_span: Option<SpanRef>,
    /// True when the value itself carries a citation.
    pub cited: bool,
    /// True when the owning `ots` component carries a citation.
    pub component_cited: bool,
    /// True when the citation that applies (its own, else its component's) is an inline note.
    pub note_only: bool,
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
///
/// Reporting level: a component none of whose values is cited (and which is not itself cited)
/// gets one diagnostic at the component path, since the one fix is to cite the component; a
/// partly cited component gets one diagnostic per uncited value, naming the gaps.
// frob:ticket 01M4YVXBMV71XB25G0Z9MJNAM5
pub fn check_s001(
    values: &[OtsValueFacts],
    locate: impl Fn(&SpanRef) -> Location,
) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    let mut reported: Vec<&str> = Vec::new();
    for v in values {
        if v.cited || v.component_cited {
            trace!(path = %v.path, "S001: cited");
            continue;
        }
        let any_cited = values.iter().any(|o| o.component == v.component && o.cited);
        if !any_cited {
            if reported.contains(&v.component.as_str()) {
                continue;
            }
            reported.push(&v.component);
            debug!(path = %v.component, "S001: ots component without any citation");
            let mut d = Diagnostic::new(
                CODE_S001,
                Severity::Warning,
                format!(
                    "ots component `{}` has no citation on it or its values",
                    v.component
                ),
            );
            d.path = Some(v.component.clone());
            d.location = v.component_span.as_ref().map(&locate);
            out.push(d);
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

/// A `cite` item whose digest differs from the acknowledged one, with the values cited to it.
#[derive(Clone, Debug)]
pub struct CitationDrift {
    /// Canonical path of the `cite` item.
    pub path: String,
    /// Canonical paths of the nodes citing it.
    pub citing: Vec<String>,
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

/// Run S004 over `drift`: acknowledged `cite` items whose digest changed since.
pub fn check_s004(drift: &[CitationDrift]) -> Vec<Diagnostic> {
    drift
        .iter()
        .map(|c| {
            debug!(path = %c.path, citing = c.citing.len(), "S004: citation changed");
            let mut d = Diagnostic::new(
                CODE_S004,
                Severity::Warning,
                format!(
                    "citation `{}` changed since it was acknowledged; re-read the source for: {}",
                    c.path,
                    c.citing.join(", ")
                ),
            );
            d.path = Some(c.path.clone());
            d
        })
        .collect()
}

/// Field groups a citation type needs; each group is satisfied by any one of its names.
fn required_fields(ty: CitationType) -> &'static [&'static [&'static str]] {
    match ty {
        CitationType::Standard => &[&["number"], &["title"]],
        CitationType::Datasheet => &[&["organization"], &["number"], &["rev", "date"]],
        CitationType::Drawing => &[&["organization"], &["number"], &["rev"]],
        CitationType::Catalog => &[&["organization"], &["title"], &["edition", "date"]],
        CitationType::Book => &[&["title"], &["publisher"], &["edition", "date"]],
        CitationType::Article => &[&["author"], &["title"], &["journal"], &["date"]],
        CitationType::Web => &[&["title"], &["url"], &["accessed"]],
        CitationType::Quote => &[&["organization"], &["number"], &["date"]],
        CitationType::Document => &[&["title"], &["number", "date"]],
    }
}

/// The fields `c` lacks, each rendered as `a` or `a or b` (a `url` without `file` needs `accessed`).
pub fn missing_fields(c: &Citation) -> Vec<String> {
    let mut missing: Vec<String> = required_fields(c.ty)
        .iter()
        .filter(|group| !group.iter().any(|f| c.fields.contains_key(*f)))
        .map(|group| group.join(" or "))
        .collect();
    let web_accessed = c.ty == CitationType::Web;
    if c.fields.contains_key("url")
        && !c.fields.contains_key("file")
        && !c.fields.contains_key("accessed")
        && !web_accessed
    {
        missing.push("accessed".to_owned());
    }
    missing
}

/// Run S008 over `cites` (incomplete records) and `values` (an `ots` value cited only by a note).
pub fn check_s008(
    cites: &[Citation],
    values: &[OtsValueFacts],
    locate: impl Fn(&SpanRef) -> Location,
) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for c in cites {
        let missing = missing_fields(c);
        if missing.is_empty() {
            trace!(path = %c.path, "S008: citation complete");
            continue;
        }
        debug!(path = %c.path, ?missing, "S008: citation incomplete");
        let mut d = Diagnostic::new(
            CODE_S008,
            Severity::Warning,
            format!(
                "citation `{}` ({}) lacks {}",
                c.path,
                c.ty.name(),
                missing
                    .iter()
                    .map(|m| format!("`{m}`"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        );
        d.path = Some(c.path.clone());
        out.push(d);
    }
    for v in values.iter().filter(|v| v.note_only) {
        debug!(path = %v.path, "S008: ots value cited only by a note");
        let mut d = Diagnostic::new(
            CODE_S008,
            Severity::Warning,
            format!(
                "ots value `{}` is cited only by an inline note; cite a source nobody has to guess",
                v.path
            ),
        );
        d.path = Some(v.path.clone());
        d.location = v.span.as_ref().map(&locate);
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
            component: "demo::Bolt".into(),
            component_span: None,
            cited,
            component_cited,
            note_only: false,
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
    fn s001_one_component_diagnostic_when_nothing_cited() {
        let d = check_s001(&[val(false, false), val(false, false)], |_| unreachable!());
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].path.as_deref(), Some("demo::Bolt"));
    }

    // frob:tests crates/cadetta-lint/src/rules/s.rs::check_s001
    #[test]
    fn s001_per_value_when_component_partly_cited() {
        let d = check_s001(&[val(true, false), val(false, false)], |_| unreachable!());
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].path.as_deref(), Some("demo::Bolt.pitch"));
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

    fn item(ty: CitationType, fields: &[(&str, &str)]) -> Citation {
        use cadetta_ir::model::CiteField;
        let f = fields
            .iter()
            .map(|(k, v)| ((*k).to_owned(), CiteField::Text((*v).to_owned())))
            .collect();
        Citation::new("demo::c", ty, f, None)
    }

    // frob:tests crates/cadetta-lint/src/rules/s.rs::check_s008
    #[test]
    fn s008_fires_per_type_missing_field() {
        let cases = [
            item(CitationType::Standard, &[("title", "T")]),
            item(
                CitationType::Datasheet,
                &[("organization", "O"), ("number", "N")],
            ),
            item(
                CitationType::Drawing,
                &[("organization", "O"), ("number", "N")],
            ),
            item(
                CitationType::Catalog,
                &[("organization", "O"), ("title", "T")],
            ),
            item(CitationType::Book, &[("title", "T"), ("publisher", "P")]),
            item(
                CitationType::Article,
                &[("author", "A"), ("title", "T"), ("journal", "J")],
            ),
            item(
                CitationType::Web,
                &[("title", "T"), ("url", "https://x.example")],
            ),
            item(
                CitationType::Quote,
                &[("organization", "O"), ("number", "N")],
            ),
            item(CitationType::Document, &[("title", "T")]),
        ];
        for c in &cases {
            let d = check_s008(std::slice::from_ref(c), &[], |_| unreachable!());
            assert_eq!(d.len(), 1, "{:?}", c.ty);
            assert_eq!(d[0].code, "S008");
            assert_eq!(d[0].severity, Severity::Warning);
        }
    }

    // frob:tests crates/cadetta-lint/src/rules/s.rs::check_s008
    #[test]
    fn s008_url_without_file_needs_accessed_but_file_or_accessed_is_enough() {
        let base = [("organization", "O"), ("number", "N"), ("date", "2024")];
        let mut with_url = base.to_vec();
        with_url.push(("url", "https://x.example/a.pdf"));
        let c = item(CitationType::Quote, &with_url);
        assert_eq!(missing_fields(&c), vec!["accessed".to_owned()]);
        with_url.push(("file", "docs/a.pdf"));
        assert!(missing_fields(&item(CitationType::Quote, &with_url)).is_empty());
        let mut with_acc = base.to_vec();
        with_acc.extend([("url", "https://x.example"), ("accessed", "2026-09-30")]);
        assert!(missing_fields(&item(CitationType::Quote, &with_acc)).is_empty());
    }

    // frob:tests crates/cadetta-lint/src/rules/s.rs::check_s008
    #[test]
    fn s008_quiet_when_complete_and_fires_on_note_only_ots() {
        let ok = item(
            CitationType::Standard,
            &[("number", "ISO 273"), ("title", "T")],
        );
        assert!(check_s008(&[ok], &[], |_| unreachable!()).is_empty());
        let mut v = val(true, false);
        assert!(check_s008(&[], std::slice::from_ref(&v), |_| unreachable!()).is_empty());
        v.note_only = true;
        let d = check_s008(&[], &[v], |_| unreachable!());
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].code, "S008");
    }

    // frob:tests crates/cadetta-lint/src/rules/s.rs::check_s004
    #[test]
    fn s004_lists_citing_values() {
        let d = check_s004(&[CitationDrift {
            path: "demo::c".into(),
            citing: vec!["demo::Bolt.pitch".into()],
        }]);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].code, "S004");
        assert!(d[0].message.contains("demo::Bolt.pitch"));
        assert!(check_s004(&[]).is_empty());
    }
}
