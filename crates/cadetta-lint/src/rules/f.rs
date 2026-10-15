//! Lints F001-F004: ISO 286 fits.
//!
//! F001 (unknown class) and F002 (size out of range) are fixed errors found
//! by `cadetta-fits`; F003 (side tolerance disagrees with its class) is found
//! by sema. [`lift`] turns those findings into catalog diagnostics. F004 is
//! derived from the IR here: a fit whose clearance lies entirely outside its
//! required range. A partial overlap is C002 and is not reported by this
//! module.
//!
//! Specification: `docs/spec/lints.md` (F), `docs/spec/language.md` 3.7,
//! `docs/spec/quantities.md` 5.1.

// frob:ticket 01M4SPWR26H443049B6E0Q9EBH

use cadetta_ir::model::{Bound, BoundOp, Fit, Nominal, Project, SpanRef, Value};
use cadetta_units::{Magnitude, Sum};
use tracing::{debug, trace, warn};

use crate::diag::{Diagnostic, Location};
use crate::engine::{LintConfig, explain};

/// Rule code: fit class not in the ISO 286 tables (fixed error).
pub const F001: &str = "F001";
/// Rule code: nominal outside the table size range (fixed error).
pub const F002: &str = "F002";
/// Rule code: a side's tolerance disagrees with its class (error).
pub const F003: &str = "F003";
/// Rule code: clearance entirely outside the required range (error).
pub const F004: &str = "F004";

/// An F finding raised by `cadetta-fits` or sema, before severity is applied.
#[derive(Clone, Debug)]
pub struct FFinding {
    /// Catalog code, `F001` to `F004`.
    pub code: String,
    /// One sentence, no trailing period.
    pub message: String,
    /// Canonical path of the fit or fitted member, if known.
    pub path: Option<String>,
    /// Where to look.
    pub location: Option<Location>,
}

/// Turn F findings into diagnostics at the catalog severity; non-F codes are dropped.
pub fn lift(findings: &[FFinding], cfg: &LintConfig) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for f in findings {
        let Some(info) = explain(&f.code).filter(|i| i.code.starts_with('F')) else {
            warn!(code = %f.code, "F: not an F-family catalog code; finding dropped");
            continue;
        };
        let Some(sev) = cfg.severity(info.code) else {
            continue;
        };
        debug!(code = %f.code, ?sev, "F: finding lifted");
        let mut d = Diagnostic::new(info.code, sev, f.message.clone());
        d.path = f.path.clone();
        d.location = f.location.clone();
        out.push(d);
    }
    out
}

/// The sign of an exact magnitude: -1, 0 or 1.
fn sign(m: &Magnitude) -> i8 {
    if m.is_zero() {
        0
    } else if m.rational().numer().to_string().starts_with('-') {
        -1
    } else {
        1
    }
}

/// The sign of `a - b`, or None when the difference is not exact.
fn cmp(a: &Magnitude, b: &Magnitude) -> Option<i8> {
    match a.sub(b) {
        Sum::Exact(m) => Some(sign(&m)),
        Sum::Inexact(_) => None,
    }
}

/// Lowest and highest value of a toleranced quantity, when exact.
pub(crate) fn bounds(v: &Value) -> Option<(Magnitude, Magnitude)> {
    let Nominal::Exact(nom) = &v.nominal else {
        return None;
    };
    let (mut lo, mut hi) = (nom.0.clone(), nom.0.clone());
    for t in v.tol.iter().flat_map(|a| a.terms.iter()) {
        let (Sum::Exact(l), Sum::Exact(h)) = (lo.add(&t.lo.0), hi.add(&t.hi.0)) else {
            return None;
        };
        lo = l;
        hi = h;
    }
    Some((lo, hi))
}

/// True when every value of the range lies inside the `bound`.
// frob:ticket 01M4V13T1DW08P87XP4J4802RH
pub(crate) fn holds(range: (&Magnitude, &Magnitude), bound: &Bound) -> bool {
    let (min, max) = range;
    let lo_ok = match (&bound.lo, bound.op) {
        (Some(lo), BoundOp::Ge | BoundOp::In) => cmp(min, &lo.0).is_some_and(|s| s >= 0),
        _ => true,
    };
    let hi_ok = match (&bound.hi, bound.op) {
        (Some(hi), BoundOp::Le | BoundOp::In) => cmp(max, &hi.0).is_some_and(|s| s <= 0),
        _ => true,
    };
    lo_ok && hi_ok
}

/// True when every clearance lies strictly outside the required `bound`.
pub(crate) fn violates(clearance: (&Magnitude, &Magnitude), bound: &Bound) -> bool {
    let (min, max) = clearance;
    let below = bound.lo.as_ref().is_some_and(|lo| {
        matches!(bound.op, BoundOp::Ge | BoundOp::In) && cmp(max, &lo.0) == Some(-1)
    });
    let above = bound.hi.as_ref().is_some_and(|hi| {
        matches!(bound.op, BoundOp::Le | BoundOp::In) && cmp(min, &hi.0) == Some(1)
    });
    below || above
}

fn check_fit(
    fit: &Fit,
    sev: crate::diag::Severity,
    locate: &impl Fn(&SpanRef) -> Location,
    span: Option<&SpanRef>,
) -> Option<Diagnostic> {
    let bound = fit.required.as_ref()?;
    let Some((min, max)) = bounds(&fit.clearance) else {
        trace!(fit = %fit.path, "F004: clearance not exact, skipped");
        return None;
    };
    if !violates((&min, &max), bound) {
        trace!(fit = %fit.path, "F004: range not violated");
        return None;
    }
    debug!(fit = %fit.path, "F004: clearance entirely outside required range");
    let mut d = Diagnostic::new(
        F004,
        sev,
        format!(
            "clearance of `{}` lies entirely outside its required range",
            fit.path
        ),
    );
    d.path = Some(fit.path.to_string());
    d.location = span.map(locate);
    Some(d)
}

/// F004: one diagnostic per fit whose clearance is entirely outside its required range.
pub fn check_f004(
    project: &Project,
    cfg: &LintConfig,
    locate: impl Fn(&SpanRef) -> Location,
) -> Vec<Diagnostic> {
    let sev = cfg.severity(F004).expect("F004 is in the catalog");
    project
        .components
        .iter()
        .flat_map(|c| c.fits.iter().map(move |f| (c, f)))
        .filter_map(|(c, f)| check_fit(f, sev, &locate, c.span.as_ref()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diag::Severity;
    use cadetta_ir::model::{
        Affine, AffineTerm, Component, ComponentClass, Exact, Flavour, PackageId,
    };
    use cadetta_ir::path::{Path, PathContext};
    use cadetta_units::{Dim, Rational};

    fn p(s: &str) -> Path {
        Path::parse(s, PathContext::General).unwrap()
    }

    fn ex(n: i64) -> Exact {
        Exact(Magnitude::new(Rational::new(n, 1000).unwrap(), 0))
    }

    /// A clearance of `nom` +- `tol` (thousandths of the base unit).
    fn clearance(nom: i64, lo: i64, hi: i64) -> Value {
        let mut v = Value::new(Nominal::Exact(ex(nom)), Dim::default(), "c");
        v.tol = Some(Affine {
            terms: vec![AffineTerm {
                var: p("demo::A.h"),
                lo: ex(lo),
                hi: ex(hi),
            }],
        });
        v
    }

    fn project(c: Value, req: Option<Bound>) -> Project {
        let mut proj = Project::new(
            Flavour::Design,
            PackageId {
                name: "demo".into(),
                version: "0.1.0".into(),
            },
        );
        proj.components.push(Component {
            path: p("demo::A.x"),
            class: ComponentClass::Model,
            params_sig: None,
            implements: vec![],
            citation: None,
            attrs: Default::default(),
            members: vec![],
            parts: vec![],
            fits: vec![Fit {
                path: p("demo::A.seat"),
                hole: p("demo::A.h"),
                shaft: p("demo::A.s"),
                clearance: c,
                fit_type: "clearance".into(),
                required: req,
            }],
            datums: vec![],
            points: vec![],
            placements: vec![],
            span: None,
            digest: None,
        });
        proj
    }

    fn range(lo: i64, hi: i64) -> Option<Bound> {
        Some(Bound {
            op: BoundOp::In,
            lo: Some(ex(lo)),
            hi: Some(ex(hi)),
        })
    }

    fn loc(_: &SpanRef) -> Location {
        unreachable!("no spans in these fixtures")
    }

    fn cfg() -> LintConfig {
        LintConfig::new()
    }

    fn finding(code: &str) -> FFinding {
        FFinding {
            code: code.into(),
            message: "m".into(),
            path: Some("demo::A.seat".into()),
            location: None,
        }
    }

    // frob:tests lift_reports_f001_unwaivable
    #[test]
    fn lift_reports_f001_unwaivable() {
        let out = lift(&[finding("F001")], &cfg());
        assert_eq!(out.len(), 1);
        assert_eq!(
            (out[0].code.as_str(), out[0].severity),
            (F001, Severity::Error)
        );
        assert!(explain(F001).unwrap().fixed, "F001 cannot be waived");
    }

    // frob:tests lift_reports_f002_unwaivable
    #[test]
    fn lift_reports_f002_unwaivable() {
        let out = lift(&[finding("F002")], &cfg());
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].code, F002);
        assert!(explain(F002).unwrap().fixed, "F002 cannot be waived");
    }

    // frob:tests lift_reports_f003
    #[test]
    fn lift_reports_f003() {
        let out = lift(&[finding("F003")], &cfg());
        assert_eq!(out.len(), 1);
        assert_eq!(
            (out[0].code.as_str(), out[0].severity),
            (F003, Severity::Error)
        );
        assert!(!explain(F003).unwrap().fixed);
    }

    // frob:tests lift_drops_non_f_codes
    #[test]
    fn lift_drops_non_f_codes() {
        assert!(lift(&[finding("Q001"), finding("F999")], &cfg()).is_empty());
    }

    // frob:tests f004_fires_when_entirely_outside
    #[test]
    fn f004_fires_when_entirely_outside() {
        // clearance 10..30 against required 40..60, and 70..90 above 40..60
        let below = project(clearance(20, -10, 10), range(40, 60));
        let out = check_f004(&below, &cfg(), loc);
        assert_eq!(out.len(), 1);
        assert_eq!(
            (out[0].code.as_str(), out[0].severity),
            (F004, Severity::Error)
        );
        let above = project(clearance(80, -10, 10), range(40, 60));
        assert_eq!(check_f004(&above, &cfg(), loc).len(), 1);
    }

    // frob:tests f004_silent_on_partial_overlap_or_hold
    #[test]
    fn f004_silent_on_partial_overlap_or_hold() {
        // partial overlap is C002, holding is fine, no range is fine
        let partial = project(clearance(50, -20, 20), range(40, 60));
        assert!(check_f004(&partial, &cfg(), loc).is_empty());
        let holds = project(clearance(50, -5, 5), range(40, 60));
        assert!(check_f004(&holds, &cfg(), loc).is_empty());
        let none = project(clearance(50, -5, 5), None);
        assert!(check_f004(&none, &cfg(), loc).is_empty());
    }
}
