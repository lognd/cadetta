//! Lints Q001-Q011: the quantity family.
//!
//! Most Q diagnostics are raised where the fact is known (the lexer for Q007,
//! sema for Q001-Q003, Q008-Q011, an adapter for Q004). This module is the
//! single place that turns those findings into catalog diagnostics with the
//! severity the engine computes (fixed codes keep their catalog severity, Q004
//! follows the run mode), and that derives Q006 from the IR. Q005 is retired
//! and never emitted.
//!
//! Specification: `docs/spec/lints.md` (Q), `docs/spec/quantities.md`.

// frob:ticket 01M4SPWQPYYT8Y5N3P5ZSG20Z9

use std::collections::BTreeMap;

use cadetta_ir::model::{Nominal, Project, SpanRef};
use tracing::{debug, trace, warn};

use crate::diag::{Diagnostic, Fix, Location};
use crate::engine::{LintConfig, explain};

/// Rule code: bare number where a quantity is required (fixed error).
pub const Q001: &str = "Q001";
/// Rule code: inexact value reported at its root operation (info).
pub const Q006: &str = "Q006";
/// Rule code: whitespace between number and unit (fixed error, fix-it).
pub const Q007: &str = "Q007";
/// Retired codes: listed in the catalog document, never emitted.
pub const RETIRED: &[&str] = &["Q005"];

/// True when `code` is a retired Q code (lints.md 3, Q).
pub fn is_retired(code: &str) -> bool {
    RETIRED.contains(&code)
}

/// A Q finding raised by the lexer, sema or an adapter, before severity is applied.
#[derive(Clone, Debug)]
pub struct QFinding {
    /// Catalog code, `Q001` to `Q011`.
    pub code: String,
    /// One sentence, no trailing period.
    pub message: String,
    /// Canonical path of the subject, if known.
    pub path: Option<String>,
    /// Where to look.
    pub location: Option<Location>,
    /// Suggested fix (Q007 always carries one).
    pub fix: Option<Fix>,
}

/// Turn findings into diagnostics at the configured severity; retired and non-Q codes are dropped.
pub fn lift(findings: &[QFinding], cfg: &LintConfig) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for f in findings {
        if is_retired(&f.code) {
            warn!(code = %f.code, "Q: retired code never emitted; finding dropped");
            continue;
        }
        let Some(info) = explain(&f.code).filter(|i| i.code.starts_with('Q')) else {
            warn!(code = %f.code, "Q: not a Q-family catalog code; finding dropped");
            continue;
        };
        let Some(sev) = cfg.severity(info.code) else {
            continue;
        };
        debug!(code = %f.code, ?sev, "Q: finding lifted");
        let mut d = Diagnostic::new(info.code, sev, f.message.clone());
        d.path = f.path.clone();
        d.location = f.location.clone();
        d.fixes = f.fix.iter().cloned().collect();
        out.push(d);
    }
    out
}

/// Q006: one info per root inexact operation, counting the members that depend on it.
pub fn check_inexact(
    project: &Project,
    cfg: &LintConfig,
    locate: impl Fn(&SpanRef) -> Location,
) -> Vec<Diagnostic> {
    let sev = cfg.severity(Q006).expect("Q006 is in the catalog");
    // root path -> number of members carrying a value rooted there
    let mut roots: BTreeMap<String, usize> = BTreeMap::new();
    for c in &project.components {
        for m in &c.members {
            let Some(Nominal::Inexact(i)) = m.value.as_ref().map(|v| &v.nominal) else {
                continue;
            };
            let root = i.root.to_string();
            let own = m.path.to_string() == root;
            let n = roots.entry(root).or_insert(0);
            if !own {
                *n += 1;
            }
        }
    }
    let mut out = Vec::new();
    for (root, dependents) in roots {
        debug!(%root, dependents, "Q006: inexact root");
        let mut d = Diagnostic::new(
            Q006,
            sev,
            format!(
                "inexact value rooted at `{root}`; {dependents} dependent value(s) are inexact too"
            ),
        );
        d.location = project
            .components
            .iter()
            .flat_map(|c| c.members.iter())
            .find(|m| m.path.to_string() == root)
            .and_then(|m| m.span.as_ref())
            .map(&locate);
        d.path = Some(root);
        out.push(d);
    }
    if out.is_empty() {
        trace!("Q006: no inexact values");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diag::{Edit, Position, Range, Severity};
    use crate::engine::Mode;
    use cadetta_ir::model::{Component, ComponentClass};
    use cadetta_ir::model::{Exact, Flavour, Inexact, Member, MemberKind, PackageId, Value};
    use cadetta_ir::path::{Path, PathContext};
    use cadetta_units::{Dim, Magnitude, Rational};

    fn p(s: &str) -> Path {
        Path::parse(s, PathContext::General).unwrap()
    }

    fn finding(code: &str) -> QFinding {
        QFinding {
            code: code.into(),
            message: "m".into(),
            path: Some("demo::Plate.w".into()),
            location: None,
            fix: None,
        }
    }

    fn member(path: &str, inexact_root: Option<&str>) -> Member {
        let mut m = Member::new(p(path), MemberKind::Param);
        let nominal = match inexact_root {
            Some(r) => Nominal::Inexact(Inexact {
                f64: 1.5,
                root: p(r),
            }),
            None => Nominal::Exact(Exact(Magnitude::new(Rational::new(1, 1).unwrap(), 0))),
        };
        m.value = Some(Value::new(nominal, Dim::default(), "x"));
        m
    }

    fn project(members: Vec<Member>) -> Project {
        let mut proj = Project::new(
            Flavour::Design,
            PackageId {
                name: "demo".into(),
                version: "0.1.0".into(),
            },
        );
        let c = Component {
            path: p("demo::Plate.w"),
            class: ComponentClass::Model,
            params_sig: None,
            implements: vec![],
            citation: None,
            attrs: Default::default(),
            members,
            parts: vec![],
            fits: vec![],
            datums: vec![],
            points: vec![],
            placements: vec![],
            span: None,
            digest: None,
        };
        proj.components.push(c);
        proj
    }

    // frob:tests crates/cadetta-lint/src/rules/q.rs::lift kind=unit
    #[test]
    fn q001_fires_as_error() {
        let d = lift(&[finding("Q001")], &LintConfig::new());
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].code, "Q001");
        assert_eq!(d[0].severity, Severity::Error);
    }

    // frob:tests crates/cadetta-lint/src/rules/q.rs::lift kind=unit
    #[test]
    fn q007_keeps_fix_and_cannot_be_waived() {
        let mut f = finding("Q007");
        let pos = Position { line: 1, col: 2 };
        f.fix = Some(Fix {
            title: "remove the space".into(),
            edits: vec![Edit {
                file: "a.cdt".into(),
                range: Range {
                    start: pos,
                    end: pos,
                },
                text: String::new(),
            }],
        });
        let d = lift(&[f], &LintConfig::new());
        assert_eq!(d[0].fixes.len(), 1);
        assert!(explain("Q007").unwrap().fixed);
    }

    // frob:tests crates/cadetta-lint/src/rules/q.rs::lift kind=unit
    #[test]
    fn fixed_codes_ignore_severity_override() {
        for code in ["Q001", "Q002", "Q003", "Q007", "Q009", "Q010", "Q011"] {
            let mut cfg = LintConfig::new();
            cfg.set_severity(code, "info").unwrap();
            let d = lift(&[finding(code)], &cfg);
            assert_eq!(d[0].severity, Severity::Error, "{code}");
        }
    }

    // frob:tests crates/cadetta-lint/src/rules/q.rs::lift kind=unit
    #[test]
    fn default_severities_of_q004_q006_q008() {
        let cfg = LintConfig::new();
        let sev = |c: &str| lift(&[finding(c)], &cfg)[0].severity;
        assert_eq!(sev("Q004"), Severity::Warning);
        assert_eq!(sev("Q006"), Severity::Info);
        assert_eq!(sev("Q008"), Severity::Warning);
        let mut sync = LintConfig::new();
        sync.mode = Mode::Sync;
        assert_eq!(lift(&[finding("Q004")], &sync)[0].severity, Severity::Error);
    }

    // frob:tests crates/cadetta-lint/src/rules/q.rs::is_retired kind=unit
    #[test]
    fn q005_is_retired_and_never_emitted() {
        assert!(is_retired("Q005"));
        assert!(explain("Q005").is_none());
        assert!(lift(&[finding("Q005")], &LintConfig::new()).is_empty());
    }

    // frob:tests crates/cadetta-lint/src/rules/q.rs::check_inexact kind=unit
    #[test]
    fn q006_once_per_root_with_dependent_count() {
        let proj = project(vec![
            member("demo::Plate.a", Some("demo::Plate.a")),
            member("demo::Plate.b", Some("demo::Plate.a")),
            member("demo::Plate.c", Some("demo::Plate.a")),
            member("demo::Plate.d", None),
        ]);
        let d = check_inexact(&proj, &LintConfig::new(), |_| Location::Lock { line: 1 });
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].severity, Severity::Info);
        assert_eq!(d[0].path.as_deref(), Some("demo::Plate.a"));
        assert!(d[0].message.contains("2 dependent"), "{}", d[0].message);
    }

    // frob:tests crates/cadetta-lint/src/rules/q.rs::check_inexact kind=unit
    #[test]
    fn q006_silent_when_all_exact() {
        let proj = project(vec![member("demo::Plate.d", None)]);
        assert!(
            check_inexact(&proj, &LintConfig::new(), |_| Location::Lock { line: 1 }).is_empty()
        );
    }
}
