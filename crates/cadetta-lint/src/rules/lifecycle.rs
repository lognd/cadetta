//! Lints L001, L002, T001 and P001: lifecycle, table rows and process roles.
//!
//! Each of these is decided upstream, where the facts live: the variants crate
//! reads `lifecycle` attributes and viable alternates (L001/L002), the table
//! evaluator sees an `ots` literal match no row (T001) and the process pass sees
//! a feature-like parameter with no role (P001). They hand findings over and the
//! `lift_*` functions here turn them into catalog diagnostics at the configured
//! severity. L001 is an error once the part's date has passed `--as-of`.
//!
//! Specification: `docs/spec/lints.md` (L, T, P), `docs/spec/language.md` 3.2, 3.6, 3.8.

// frob:ticket 01M4STCCQD6C55WEN7HFEBPEYV

use tracing::{debug, trace};

use crate::diag::{Diagnostic, Location, Severity};
use crate::engine::LintConfig;

/// Rule code: an end-of-life primary with no viable alternate.
pub const L001: &str = "L001";
/// Rule code: a not-recommended-for-new-design primary.
pub const L002: &str = "L002";
/// Rule code: an `ots` literal matches no row of the table it cites.
pub const T001: &str = "T001";
/// Rule code: a feature-like parameter under a process has no rule role.
pub const P001: &str = "P001";

/// A lifecycle finding about one primary (L001 or L002).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LifecycleFinding {
    /// `L001` or `L002`.
    pub code: String,
    /// Canonical path of the primary part.
    pub path: String,
    /// The status word (`eol`, `obsolete`, `nrnd`).
    pub status: String,
    /// The lifecycle date as written.
    pub date: String,
    /// True when the date has passed the `--as-of` date.
    pub date_passed: bool,
    /// Viable alternates that could replace the primary.
    pub candidates: Vec<String>,
    /// Where the lifecycle attribute is.
    pub location: Option<Location>,
}

/// An `ots` literal that matches no row of its cited table (T001).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableMiss {
    /// Canonical path of the member holding the literal.
    pub path: String,
    /// The table cited.
    pub table: String,
    /// The column the literal was compared against.
    pub column: String,
    /// Where the literal is written.
    pub location: Option<Location>,
}

/// A feature-like parameter under a process with no rule role (P001).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissingRole {
    /// Canonical path of the parameter.
    pub path: String,
    /// The process in force, when known.
    pub process: Option<String>,
    /// Where the parameter is declared.
    pub location: Option<Location>,
}

/// Build a diagnostic at `path` and `location`.
fn diag(
    code: &str,
    sev: Severity,
    msg: String,
    path: &str,
    location: &Option<Location>,
) -> Diagnostic {
    let mut d = Diagnostic::new(code, sev, msg);
    d.path = Some(path.to_owned());
    d.location = location.clone();
    d
}

/// L001 and L002: one diagnostic per finding; L001 is an error once its date has passed.
pub fn lift_lifecycle(findings: &[LifecycleFinding], cfg: &LintConfig) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for f in findings {
        let code = match f.code.as_str() {
            L001 => L001,
            L002 => L002,
            other => {
                trace!(code = other, "L: not a lifecycle code");
                continue;
            }
        };
        let Some(mut sev) = cfg.severity(code) else {
            trace!(code, "L: rule off");
            continue;
        };
        let msg = if code == L001 {
            if f.date_passed {
                sev = Severity::Error;
            }
            format!(
                "`{}` is {} as of {}{} and has no viable alternate",
                f.path,
                f.status,
                f.date,
                if f.date_passed { " (date passed)" } else { "" }
            )
        } else {
            let alts = if f.candidates.is_empty() {
                String::new()
            } else {
                format!("; viable alternates: {}", f.candidates.join(", "))
            };
            format!(
                "`{}` is {} (not recommended for new designs) since {}{alts}",
                f.path, f.status, f.date
            )
        };
        debug!(code, path = %f.path, ?sev, "L: lifecycle diagnostic");
        out.push(diag(code, sev, msg, &f.path, &f.location));
    }
    out
}

/// T001: one diagnostic per `ots` literal that matches no row.
pub fn lift_not_in_table(misses: &[TableMiss], cfg: &LintConfig) -> Vec<Diagnostic> {
    let Some(sev) = cfg.severity(T001) else {
        return Vec::new();
    };
    misses
        .iter()
        .map(|m| {
            debug!(path = %m.path, table = %m.table, "T001: literal matches no row");
            diag(
                T001,
                sev,
                format!(
                    "literal matches no row of `{}` column `{}`",
                    m.table, m.column
                ),
                &m.path,
                &m.location,
            )
        })
        .collect()
}

/// P001: one diagnostic per feature-like parameter lacking a rule role.
pub fn lift_missing_role(missing: &[MissingRole], cfg: &LintConfig) -> Vec<Diagnostic> {
    let Some(sev) = cfg.severity(P001) else {
        return Vec::new();
    };
    missing
        .iter()
        .map(|m| {
            debug!(path = %m.path, "P001: missing process role");
            let under = m
                .process
                .as_ref()
                .map(|p| format!(" under process `{p}`"))
                .unwrap_or_default();
            diag(
                P001,
                sev,
                format!("`{}` names a feature but has no rule role{under}", m.path),
                &m.path,
                &m.location,
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn finding(code: &str, passed: bool) -> LifecycleFinding {
        LifecycleFinding {
            code: code.into(),
            path: "demo::Motor".into(),
            status: if code == L001 { "eol" } else { "nrnd" }.into(),
            date: "2026-01".into(),
            date_passed: passed,
            candidates: vec![],
            location: None,
        }
    }

    // frob:tests crates/cadetta-lint/src/rules/lifecycle.rs::lift_lifecycle
    #[test]
    fn l001_warns_before_and_errors_after_the_date() {
        let cfg = LintConfig::new();
        let before = lift_lifecycle(&[finding(L001, false)], &cfg);
        assert_eq!(before[0].code, "L001");
        assert_eq!(before[0].severity, Severity::Warning);
        let after = lift_lifecycle(&[finding(L001, true)], &cfg);
        assert_eq!(after[0].severity, Severity::Error);
        assert!(after[0].message.contains("date passed"));
    }

    // frob:tests crates/cadetta-lint/src/rules/lifecycle.rs::lift_lifecycle
    #[test]
    fn l002_is_info_and_lists_alternates() {
        let mut f = finding(L002, true);
        f.candidates = vec!["Colson".into()];
        let d = lift_lifecycle(&[f], &LintConfig::new());
        assert_eq!(d[0].code, "L002");
        assert_eq!(d[0].severity, Severity::Info);
        assert!(d[0].message.contains("Colson"));
        assert!(lift_lifecycle(&[finding("X999", false)], &LintConfig::new()).is_empty());
    }

    // frob:tests crates/cadetta-lint/src/rules/lifecycle.rs::lift_not_in_table
    #[test]
    fn t001_lifts_table_miss() {
        let m = [TableMiss {
            path: "demo::Screw.pitch".into(),
            table: "Iso261".into(),
            column: "coarse".into(),
            location: None,
        }];
        let d = lift_not_in_table(&m, &LintConfig::new());
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].code, "T001");
        assert_eq!(d[0].severity, Severity::Warning);
        assert!(lift_not_in_table(&[], &LintConfig::new()).is_empty());
    }

    // frob:tests crates/cadetta-lint/src/rules/lifecycle.rs::lift_missing_role
    #[test]
    fn p001_lifts_missing_role() {
        let m = [MissingRole {
            path: "demo::Case.slot_width".into(),
            process: Some("Fdm".into()),
            location: None,
        }];
        let d = lift_missing_role(&m, &LintConfig::new());
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].code, "P001");
        assert_eq!(d[0].severity, Severity::Warning);
        assert!(d[0].message.contains("Fdm"));
    }
}
