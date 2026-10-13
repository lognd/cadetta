// frob:ticket ~ZK9NP4M
//! Three-valued comparison over affine forms (quantities.md 5.8).
//!
//! A comparison yields `holds`, `violates` or `undecided`; undecided maps
//! to C002 (and says so when conservative, 5.4), and `==` on raw toleranced
//! values maps to C003, which suggests the accessor form.

use tracing::{debug, trace};

use crate::dimension::Dim;
use crate::rational::Magnitude;
use crate::tolerance::{Affine, InexactInterval, ToleranceError, Toleranced, VarId};

/// Relative width under which an inexact difference is undecided (5.8).
pub const INEXACT_REL_TOL: f64 = 1e-9;

/// A comparison operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CmpOp {
    /// `<`
    Lt,
    /// `<=`
    Le,
    /// `>`
    Gt,
    /// `>=`
    Ge,
    /// `==`
    Eq,
    /// `!=`
    Ne,
}

/// The three-valued outcome of a comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// True for every value within tolerance.
    Holds,
    /// False for every value within tolerance.
    Violates,
    /// True for some values and false for others (C002).
    Undecided,
}

/// Why a verdict carries a caveat.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Note {
    /// An operand came from a nonlinear fallback, so the bounds are outer.
    Conservative,
    /// An operand is inexact and the difference is within 1e-9 relative.
    Inexact,
}

/// A comparison operand: an exact-arithmetic form or a binary64 interval.
#[derive(Debug, Clone)]
pub enum Operand {
    /// An affine form with exact bounds.
    Form(Affine),
    /// An inexact (binary64) interval of the given dimension.
    Interval(Dim, InexactInterval),
}

impl Operand {
    fn dim(&self) -> &Dim {
        match self {
            Operand::Form(a) => a.dim(),
            Operand::Interval(d, _) => d,
        }
    }

    fn is_exact(&self) -> bool {
        matches!(self, Operand::Form(a) if a.is_exact())
    }

    fn is_conservative(&self) -> bool {
        match self {
            Operand::Form(a) => a.is_conservative(),
            Operand::Interval(_, i) => i.conservative,
        }
    }

    fn f64_bounds(&self) -> Result<(f64, f64), ToleranceError> {
        match self {
            Operand::Form(a) => {
                let (lo, hi) = a.bounds()?;
                Ok((lo.to_f64(), hi.to_f64()))
            }
            Operand::Interval(_, i) => Ok((i.lo, i.hi)),
        }
    }
}

/// A diagnostic produced by a comparison (C002 or C003).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompareDiagnostic {
    /// The rule id, `C002` or `C003`.
    pub code: &'static str,
    /// Human-readable message.
    pub message: String,
    /// Suggested replacement text, when one exists (C003).
    pub fix: Option<String>,
}

/// The result of one comparison.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Comparison {
    /// The three-valued verdict.
    pub verdict: Verdict,
    /// Caveats on the verdict.
    pub notes: Vec<Note>,
    /// True when `==`/`!=` was written on raw toleranced values (C003).
    pub toleranced_equality: bool,
}

impl Comparison {
    /// The diagnostics this result reports: C002 if undecided, C003 for raw `==`.
    pub fn diagnostics(&self) -> Vec<CompareDiagnostic> {
        let mut out = Vec::new();
        if self.verdict == Verdict::Undecided {
            let mut message = String::from("comparison is undecided over the tolerance range");
            if self.notes.contains(&Note::Conservative) {
                message.push_str("; the evaluation is conservative (nonlinear fallback)");
            }
            if self.notes.contains(&Note::Inexact) {
                message
                    .push_str("; an operand is inexact and the difference is within 1e-9 relative");
            }
            out.push(CompareDiagnostic {
                code: "C002",
                message,
                fix: None,
            });
        }
        if self.toleranced_equality {
            out.push(CompareDiagnostic {
                code: "C003",
                message: "`==` on toleranced values is almost always undecided".into(),
                fix: Some("compare `a.nominal == b.nominal`".into()),
            });
        }
        out
    }
}

fn decide(op: CmpOp, lo: f64, hi: f64) -> Verdict {
    use Verdict::*;
    match op {
        CmpOp::Gt => pick(lo > 0.0, hi <= 0.0),
        CmpOp::Ge => pick(lo >= 0.0, hi < 0.0),
        CmpOp::Lt => pick(hi < 0.0, lo >= 0.0),
        CmpOp::Le => pick(hi <= 0.0, lo > 0.0),
        CmpOp::Eq => pick(lo == 0.0 && hi == 0.0, lo > 0.0 || hi < 0.0),
        CmpOp::Ne => match decide(CmpOp::Eq, lo, hi) {
            Holds => Violates,
            Violates => Holds,
            Undecided => Undecided,
        },
    }
}

fn pick(holds: bool, violates: bool) -> Verdict {
    if holds {
        Verdict::Holds
    } else if violates {
        Verdict::Violates
    } else {
        Verdict::Undecided
    }
}

fn sign_of(m: &Magnitude) -> i8 {
    use num_bigint::Sign;
    match m.rational().numer().sign() {
        Sign::Minus => -1,
        Sign::NoSign => 0,
        Sign::Plus => 1,
    }
}

/// Compare `a op b` over every value in tolerance (quantities.md 5.8).
///
/// Forms are subtracted symbolically so shared variables cancel; exact
/// comparisons are exact. Mixed-pi-exponent or binary64 operands use
/// binary64 and report undecided within 1e-9 relative.
pub fn compare(op: CmpOp, a: &Operand, b: &Operand) -> Result<Comparison, ToleranceError> {
    if a.dim() != b.dim() {
        return Err(ToleranceError::DimensionMismatch {
            left: format!("{:?}", a.dim()),
            right: format!("{:?}", b.dim()),
        });
    }
    let mut notes = Vec::new();
    let exact_diff = match (a, b) {
        (Operand::Form(x), Operand::Form(y)) => match x.sub(y) {
            Ok(d) => Some(d),
            Err(ToleranceError::MixedPiExponent) => None,
            Err(e) => return Err(e),
        },
        _ => None,
    };
    let verdict = if let Some(d) = exact_diff {
        let (lo, hi) = d.bounds()?;
        let (ls, hs) = (sign_of(&lo), sign_of(&hi));
        let v = decide_signs(op, ls, hs);
        trace!(?op, ls, hs, ?v, "compare: exact difference");
        v
    } else {
        let (alo, ahi) = a.f64_bounds()?;
        let (blo, bhi) = b.f64_bounds()?;
        let scale = alo.abs().max(ahi.abs()).max(blo.abs()).max(bhi.abs());
        let tol = INEXACT_REL_TOL * scale;
        let (lo, hi) = (alo - bhi - tol, ahi - blo + tol);
        let v = decide(op, lo, hi);
        if v == Verdict::Undecided {
            notes.push(Note::Inexact);
        }
        trace!(?op, lo, hi, ?v, "compare: inexact difference");
        v
    };
    if verdict == Verdict::Undecided && (a.is_conservative() || b.is_conservative()) {
        notes.push(Note::Conservative);
    }
    let toleranced_equality =
        matches!(op, CmpOp::Eq | CmpOp::Ne) && !(a.is_exact() && b.is_exact());
    debug!(?op, ?verdict, ?notes, toleranced_equality, "compare: done");
    Ok(Comparison {
        verdict,
        notes,
        toleranced_equality,
    })
}

/// Decide from exact signs (-1, 0, 1) of the difference's bounds.
fn decide_signs(op: CmpOp, ls: i8, hs: i8) -> Verdict {
    // Map to the f64 decision table on {-1,0,1} values; exact, no rounding.
    decide(op, f64::from(ls), f64::from(hs))
}

/// Compare nominals exactly (`a.nominal == b.nominal`); never raises C003.
pub fn compare_nominals(
    op: CmpOp,
    a: &Toleranced,
    b: &Toleranced,
) -> Result<Comparison, ToleranceError> {
    let ca = Affine::constant(a.dim().clone(), a.nominal().clone());
    let cb = Affine::constant(b.dim().clone(), b.nominal().clone());
    compare(op, &Operand::Form(ca), &Operand::Form(cb))
}

/// Lift a toleranced value to an operand using `var` as its source variable.
pub fn operand_of(t: &Toleranced, var: VarId) -> Result<Operand, ToleranceError> {
    Ok(Operand::Form(t.to_affine(var)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dimension::Dimension;
    use crate::rational::Rational;

    fn len() -> Dim {
        Dim::new(Dimension::from_si([1, 0, 0, 0, 0, 0, 0]), 0)
    }

    fn mm(n: i64) -> Magnitude {
        Magnitude::new(Rational::new(n, 1000).unwrap(), 0)
    }

    fn tol(nom: i64, dev: i64) -> Toleranced {
        Toleranced::symmetric(len(), mm(nom), (len(), mm(dev))).unwrap()
    }

    fn form(nom: i64, dev: i64, v: u32) -> Operand {
        operand_of(&tol(nom, dev), VarId(v)).unwrap()
    }

    fn konst(n: i64) -> Operand {
        Operand::Form(Affine::constant(len(), mm(n)))
    }

    // frob:tests crates/cadetta-units/src/compare.rs::compare kind=unit
    #[test]
    fn form_entirely_above_threshold_holds() {
        let c = compare(CmpOp::Gt, &form(10, 1, 0), &konst(5)).unwrap();
        assert_eq!(c.verdict, Verdict::Holds);
        let c = compare(CmpOp::Lt, &form(10, 1, 0), &konst(5)).unwrap();
        assert_eq!(c.verdict, Verdict::Violates);
        assert!(c.diagnostics().is_empty());
    }

    // frob:tests crates/cadetta-units/src/compare.rs::compare kind=unit
    #[test]
    fn straddling_form_is_undecided() {
        let c = compare(CmpOp::Gt, &form(10, 1, 0), &konst(10)).unwrap();
        assert_eq!(c.verdict, Verdict::Undecided);
        assert_eq!(c.diagnostics()[0].code, "C002");
        // shared variable cancels: x - x is exactly zero
        let c = compare(CmpOp::Ge, &form(10, 1, 0), &form(10, 1, 0)).unwrap();
        assert_eq!(c.verdict, Verdict::Holds);
        // independent variables do not cancel
        let c = compare(CmpOp::Gt, &form(10, 1, 0), &form(10, 1, 1)).unwrap();
        assert_eq!(c.verdict, Verdict::Undecided);
    }

    // frob:tests crates/cadetta-units/src/compare.rs::Comparison.diagnostics kind=unit
    #[test]
    fn conservative_undecided_says_so_in_c002() {
        let x = tol(10, 1).to_affine(VarId(0)).unwrap();
        let sq = x.mul(&x).unwrap();
        assert!(sq.is_conservative());
        // x*x spans a range around 1e-4 m^2; compare against its own centre.
        let dim = sq.dim().clone();
        let thr = Operand::Form(Affine::constant(dim, sq.center().clone()));
        let c = compare(CmpOp::Gt, &Operand::Form(sq), &thr).unwrap();
        assert_eq!(c.verdict, Verdict::Undecided);
        assert!(c.notes.contains(&Note::Conservative));
        let d = &c.diagnostics()[0];
        assert_eq!(d.code, "C002");
        assert!(d.message.contains("conservative"));
    }

    // frob:tests crates/cadetta-units/src/compare.rs::compare kind=unit
    #[test]
    fn inexact_near_equal_is_undecided_with_note() {
        let near = Operand::Interval(
            len(),
            InexactInterval {
                lo: 0.005 * (1.0 + 1e-12),
                hi: 0.005 * (1.0 + 1e-12),
                conservative: false,
            },
        );
        let c = compare(CmpOp::Gt, &near, &konst(5)).unwrap();
        assert_eq!(c.verdict, Verdict::Undecided);
        assert!(c.notes.contains(&Note::Inexact));
        assert!(c.diagnostics()[0].message.contains("inexact"));
        let far = Operand::Interval(
            len(),
            InexactInterval {
                lo: 0.006,
                hi: 0.006,
                conservative: false,
            },
        );
        assert_eq!(
            compare(CmpOp::Gt, &far, &konst(5)).unwrap().verdict,
            Verdict::Holds
        );
    }

    // frob:tests crates/cadetta-units/src/compare.rs::compare_nominals kind=unit
    #[test]
    fn raw_equality_raises_c003_and_nominal_equality_is_exact() {
        let c = compare(CmpOp::Eq, &form(10, 1, 0), &form(10, 1, 1)).unwrap();
        assert_eq!(c.verdict, Verdict::Undecided);
        let codes: Vec<_> = c.diagnostics().iter().map(|d| d.code).collect();
        assert_eq!(codes, ["C002", "C003"]);
        assert!(
            c.diagnostics()[1]
                .fix
                .as_ref()
                .unwrap()
                .contains(".nominal")
        );
        let n = compare_nominals(CmpOp::Eq, &tol(10, 1), &tol(10, 2)).unwrap();
        assert_eq!(n.verdict, Verdict::Holds);
        assert!(n.diagnostics().is_empty());
        let n = compare_nominals(CmpOp::Eq, &tol(10, 1), &tol(11, 1)).unwrap();
        assert_eq!(n.verdict, Verdict::Violates);
    }

    // frob:tests crates/cadetta-units/src/compare.rs::compare kind=unit
    #[test]
    fn dimension_mismatch_is_an_error() {
        let mass = Dim::new(Dimension::from_si([0, 1, 0, 0, 0, 0, 0]), 0);
        let m = Operand::Form(Affine::constant(mass, mm(1)));
        assert!(compare(CmpOp::Gt, &konst(1), &m).is_err());
    }
}
