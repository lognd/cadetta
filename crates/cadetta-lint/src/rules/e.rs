//! Lints E001-E011: resolution and evaluation errors.
//!
//! Every E code is a fixed error decided upstream, where the facts live: the
//! parser (E001), name resolution (E002-E004, E011), the package loader (E005),
//! argument binding (E006), the table evaluator (E007, E008), match checking
//! (E009) and tolerance evaluation (E010). Those passes hand over plain
//! `EFinding`s and `lift_resolution` turns them into catalog diagnostics. E004
//! carries both import spans; E003 carries both definition spans.
//!
//! Specification: `docs/spec/lints.md` (3 E), `docs/spec/language.md` 3.1,
//! `docs/spec/packages.md` 1.

// frob:ticket 01M4SWKAAVAK4WGP5RYZB8MA5V

use tracing::{debug, trace};

use crate::diag::{Diagnostic, Location, Note, Severity};
use crate::engine::LintConfig;

/// Rule code: the source does not lex or parse.
pub const E001: &str = "E001";
/// Rule code: a name, path or `use` target resolves to nothing.
pub const E002: &str = "E002";
/// Rule code: two definitions share one canonical path.
pub const E003: &str = "E003";
/// Rule code: two imports bring the same used name.
pub const E004: &str = "E004";
/// Rule code: a package's `src` or manifest is missing.
pub const E005: &str = "E005";
/// Rule code: arguments do not match the parameters.
pub const E006: &str = "E006";
/// Rule code: a table lookup key matched no row.
pub const E007: &str = "E007";
/// Rule code: a table row or column does not match the table shape.
pub const E008: &str = "E008";
/// Rule code: a `match` leaves a case unhandled.
pub const E009: &str = "E009";
/// Rule code: `mmc` or `lmc` read on a tolerance with no role.
pub const E010: &str = "E010";
/// Rule code: `Type.member` read where the member depends on parameters.
pub const E011: &str = "E011";

/// One E-family trigger reported by an upstream pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EFinding {
    /// One of `E001` .. `E011`.
    pub code: String,
    /// Canonical path or file the finding is about (the subject).
    pub path: String,
    /// Pass-specific detail (the name, key, table or reason).
    pub detail: String,
    /// Where the trigger is.
    pub location: Option<Location>,
    /// Further spans (second definition, second import).
    pub related: Vec<Location>,
}

impl EFinding {
    /// Build a finding with no related spans.
    pub fn new(code: &str, path: &str, detail: &str, location: Option<Location>) -> Self {
        Self {
            code: code.to_owned(),
            path: path.to_owned(),
            detail: detail.to_owned(),
            location,
            related: Vec::new(),
        }
    }

    /// Attach a related span (E003 second definition, E004 second import).
    pub fn with_related(mut self, loc: Location) -> Self {
        self.related.push(loc);
        self
    }
}

/// The message template for `code`, or `None` when it is not an E code.
fn message(f: &EFinding) -> Option<String> {
    let (p, d) = (&f.path, &f.detail);
    Some(match f.code.as_str() {
        E001 => format!("syntax error in `{p}`: {d}"),
        E002 => format!("`{d}` in `{p}` resolves to nothing"),
        E003 => format!("`{p}` is defined more than once"),
        E004 => format!("`{d}` in `{p}` is ambiguous: brought in by two imports"),
        E005 => format!("missing package at `{p}`: {d}"),
        E006 => format!("arguments to `{p}` do not match its parameters: {d}"),
        E007 => format!("lookup key {d} matched no row of `{p}`"),
        E008 => format!("table `{p}` shape mismatch: {d}"),
        E009 => format!("`match` in `{p}` does not handle {d}"),
        E010 => format!("`{d}` read on `{p}`, a tolerance with no role (hole or boss)"),
        E011 => {
            format!("`{d}` depends on component parameters; read it from an instance, not `{p}`")
        }
        _ => return None,
    })
}

/// E001-E011: one fixed-error diagnostic per finding, with related spans noted.
pub fn lift_resolution(findings: &[EFinding], cfg: &LintConfig) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for f in findings {
        let Some(msg) = message(f) else {
            trace!(code = %f.code, "E: not an E code");
            continue;
        };
        if cfg.severity(&f.code).is_none() {
            trace!(code = %f.code, "E: rule off");
            continue;
        }
        debug!(code = %f.code, path = %f.path, "E: resolution diagnostic");
        let mut d = Diagnostic::new(&f.code, Severity::Error, msg);
        d.path = Some(f.path.clone());
        d.location = f.location.clone();
        for r in &f.related {
            d.notes.push(Note {
                message: "also here".to_owned(),
                location: Some(r.clone()),
            });
        }
        out.push(d);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lift(code: &str) -> Vec<Diagnostic> {
        lift_resolution(
            &[EFinding::new(code, "demo::A", "x", None)],
            &LintConfig::new(),
        )
    }

    // frob:tests crates/cadetta-lint/src/rules/e.rs::lift_resolution
    #[test]
    fn every_e_code_fires_as_an_error() {
        for n in 1..=11 {
            let code = format!("E{n:03}");
            let d = lift(&code);
            assert_eq!(d.len(), 1, "{code}");
            assert_eq!(d[0].code, code);
            assert_eq!(d[0].severity, Severity::Error);
        }
    }

    // frob:tests crates/cadetta-lint/src/rules/e.rs::lift_resolution
    #[test]
    fn e005_missing_package_is_an_error() {
        let d = lift(E005);
        assert_eq!(d[0].severity, Severity::Error);
        assert!(d[0].message.contains("missing package"));
    }

    // frob:tests crates/cadetta-lint/src/rules/e.rs::lift_resolution
    #[test]
    fn e004_and_e011_carry_spans() {
        let loc = Location::Lock { line: 3 };
        let f = [
            EFinding::new(E004, "demo::m", "Bolt", Some(loc.clone())).with_related(loc.clone()),
            EFinding::new(E011, "demo::Case", "Case.width", Some(loc)),
        ];
        let d = lift_resolution(&f, &LintConfig::new());
        assert_eq!(d.len(), 2);
        assert_eq!(d[0].notes.len(), 1);
        assert!(d.iter().all(|x| x.location.is_some()));
    }

    // frob:tests crates/cadetta-lint/src/rules/e.rs::lift_resolution
    #[test]
    fn e010_fires_for_roleless_mmc() {
        let d = lift_resolution(
            &[EFinding::new(E010, "demo::Pin.tol", "mmc", None)],
            &LintConfig::new(),
        );
        assert_eq!(d[0].code, "E010");
        assert!(d[0].message.contains("mmc"));
        assert!(lift("Z999").is_empty());
    }
}
