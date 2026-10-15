// frob:ticket 01M4SPWQ0WTDXX64VS40QGESR5
//! Fit declarations: sides, clearance and fit type (language.md 3.7).
//!
//! `NAME.hole`, `NAME.shaft`, `NAME.clearance` and `NAME.type` are computed
//! here from already-evaluated tolerances and never authored. The module is
//! pure: the evaluator supplies the value of each path the fit names (see
//! [`analyze`]) and stores the [`FitResult`] under the fit's name. F001 and
//! F002 come from `cadetta-fits`; F003, F004 and C002 are reported in
//! [`FitResult::diagnostics`].

use cadetta_fits::{FitError, Resolved, Side};
use cadetta_syntax::ast::FitSpec;
use cadetta_units::{
    Affine, CmpOp, Dim, Magnitude, Operand, Rational, Role, RoleInfo, RoleSource, Sum,
    ToleranceError, Toleranced, VarId, Verdict, compare, compare::operand_of,
};
use tracing::{debug, trace, warn};

/// Nanometres per metre.
const NM_PER_M: i64 = 1_000_000_000;

/// The code for a fitted side whose tolerance disagrees with its class (lints.md F003).
pub const F003: &str = "F003";
/// The code for a clearance entirely outside the required range (lints.md F004).
pub const F004: &str = "F004";
/// The code for a clearance that only partly meets the required range (lints.md C002).
pub const C002: &str = "C002";

/// The computed type of a fit, from the clearance interval (language.md 3.7).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FitType {
    /// Every clearance is at least zero.
    Clearance,
    /// The clearance spans zero.
    Transition,
    /// Every clearance is at most zero (and some is negative).
    Interference,
}

impl FitType {
    /// The source-level name (`clearance`, `transition`, `interference`).
    pub fn name(self) -> &'static str {
        match self {
            Self::Clearance => "clearance",
            Self::Transition => "transition",
            Self::Interference => "interference",
        }
    }
}

/// One side of a fit: its tolerance and the role it plays (quantities.md 5.6).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FitSide {
    /// The toleranced size of the side.
    pub value: Toleranced,
    /// Hole or shaft, with provenance.
    pub role: RoleInfo,
}

/// A non-fatal finding about a fit, with its lint code.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FitDiagnostic {
    /// The rule id (`F003`, `F004`, `C002`, or a `cadetta-fits` code).
    pub code: &'static str,
    /// Human-readable message.
    pub message: String,
}

/// The evaluated members of a fit.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FitResult {
    /// `NAME.hole`.
    pub hole: FitSide,
    /// `NAME.shaft`.
    pub shaft: FitSide,
    /// `NAME.clearance`, hole minus shaft; negative is interference.
    pub clearance: Toleranced,
    /// `NAME.type`.
    pub fit_type: FitType,
    /// F003, F004 and C002 findings.
    pub diagnostics: Vec<FitDiagnostic>,
}

/// Why a fit could not be evaluated at all.
#[derive(Clone, PartialEq, Debug)]
pub enum FitFailure {
    /// An ISO 286 lookup failed (F001 or F002).
    Iso(FitError),
    /// The nominal is not a whole number of nanometres, so it has no table row (F002).
    NominalNotNanometres,
    /// A `a / b` fit was given no second operand, or a class fit was given one.
    OperandMismatch,
    /// Tolerance arithmetic failed (dimension mismatch, mixed pi exponents).
    Tolerance(ToleranceError),
}

impl FitFailure {
    /// The lint code this failure is reported as, when it has one.
    pub fn code(&self) -> Option<&'static str> {
        match self {
            Self::Iso(e) => Some(e.code()),
            Self::NominalNotNanometres => Some("F002"),
            Self::OperandMismatch | Self::Tolerance(_) => None,
        }
    }
}

/// The human message of a failure (the lookup text, or the nanometre explanation).
pub(crate) fn failure_message(e: &FitFailure) -> String {
    match e {
        FitFailure::Iso(inner) => inner.to_string(),
        FitFailure::NominalNotNanometres => {
            "F002: the nominal size is not a whole number of nanometres".to_owned()
        }
        other => format!("{other:?}"),
    }
}

impl From<ToleranceError> for FitFailure {
    fn from(e: ToleranceError) -> Self {
        Self::Tolerance(e)
    }
}

impl From<FitError> for FitFailure {
    fn from(e: FitError) -> Self {
        Self::Iso(e)
    }
}

fn nm_magnitude(nm: i64) -> Result<Magnitude, ToleranceError> {
    let r = Rational::new(nm, NM_PER_M)?;
    Ok(Magnitude::new(r, 0))
}

fn exact(sum: Sum) -> Result<Magnitude, ToleranceError> {
    match sum {
        Sum::Exact(m) => Ok(m),
        Sum::Inexact(_) => Err(ToleranceError::MixedPiExponent),
    }
}

/// The nominal as an integer nanometre count, if it is one.
fn nominal_nm(nominal: &Magnitude) -> Option<i64> {
    if nominal.pi_exp() != 0 {
        return None;
    }
    let scaled = nominal.rational().mul(&Rational::from_int(NM_PER_M));
    let n = i64::try_from(scaled.numer()).ok()?;
    scaled.sub(&Rational::from_int(n)).is_zero().then_some(n)
}

/// Applies a resolved ISO tolerance to a nominal.
fn apply(dim: &Dim, nominal: &Magnitude, r: &Resolved) -> Result<Toleranced, ToleranceError> {
    Toleranced::from_deviations(
        dim.clone(),
        nominal.clone(),
        (dim.clone(), nm_magnitude(r.lower_nm)?),
        (dim.clone(), nm_magnitude(r.upper_nm)?),
    )
}

pub(crate) fn role_of(side: Side) -> Role {
    match side {
        Side::Hole => Role::Hole,
        Side::Shaft => Role::Shaft,
    }
}

fn info(role: Role, source: RoleSource) -> RoleInfo {
    RoleInfo { role, source }
}

/// A class applied to a nominal: the toleranced side and its resolved deviations.
pub(crate) fn class_side(
    dim: &Dim,
    nominal: &Magnitude,
    class: &str,
) -> Result<(FitSide, Resolved), FitFailure> {
    let nm = nominal_nm(nominal).ok_or_else(|| {
        warn!("F002 fit nominal is not a whole number of nanometres");
        FitFailure::NominalNotNanometres
    })?;
    let resolved = cadetta_fits::resolve(nm, class)?;
    let value = apply(dim, nominal, &resolved)?;
    let side = FitSide {
        value,
        role: info(role_of(resolved.side), RoleSource::IsoClass),
    };
    Ok((side, resolved))
}

/// True when `t` carries no tolerance of its own.
fn untoleranced(t: &Toleranced) -> bool {
    t.lower().is_zero() && t.upper().is_zero()
}

/// True when `t`'s deviations are exactly those of `r`.
fn matches_class(t: &Toleranced, r: &Resolved) -> Result<bool, ToleranceError> {
    Ok(*t.lower() == nm_magnitude(r.lower_nm)? && *t.upper() == nm_magnitude(r.upper_nm)?)
}

fn fit_type(clearance: &Toleranced) -> Result<FitType, ToleranceError> {
    let c = operand_of(clearance, VarId(0))?;
    let zero = Operand::Form(Affine::constant(
        clearance.dim().clone(),
        Magnitude::new(Rational::from_int(0), 0),
    ));
    let ty = if compare(CmpOp::Ge, &c, &zero)?.verdict == Verdict::Holds {
        FitType::Clearance
    } else if compare(CmpOp::Le, &c, &zero)?.verdict == Verdict::Holds {
        FitType::Interference
    } else {
        FitType::Transition
    };
    trace!(?ty, "fit type from clearance interval");
    Ok(ty)
}

/// Clearance of a hole and a shaft: `hole - shaft` over independent variables.
fn clearance_of(hole: &Toleranced, shaft: &Toleranced) -> Result<Toleranced, ToleranceError> {
    let lo = exact(hole.min()?.sub(&shaft.max()?))?;
    let hi = exact(hole.max()?.sub(&shaft.min()?))?;
    let nominal = exact(hole.nominal().sub(shaft.nominal()))?;
    let dim = hole.dim().clone();
    let dev_lo = exact(lo.sub(&nominal))?;
    let dev_hi = exact(hi.sub(&nominal))?;
    Toleranced::from_deviations(dim.clone(), nominal, (dim.clone(), dev_lo), (dim, dev_hi))
}

/// Checks the clearance against a required range; F004 or C002 when it fails.
fn check_range(
    clearance: &Toleranced,
    lo: &Toleranced,
    hi: &Toleranced,
) -> Result<Option<FitDiagnostic>, ToleranceError> {
    let c = operand_of(clearance, VarId(0))?;
    let lo_op: Operand = operand_of(lo, VarId(1))?;
    let hi_op: Operand = operand_of(hi, VarId(2))?;
    let above = compare(CmpOp::Ge, &c, &lo_op)?.verdict;
    let below = compare(CmpOp::Le, &c, &hi_op)?.verdict;
    debug!(?above, ?below, "fit required range verdicts");
    Ok(match (above, below) {
        (Verdict::Violates, _) | (_, Verdict::Violates) => Some(FitDiagnostic {
            code: F004,
            message: "clearance lies entirely outside the required range".to_owned(),
        }),
        (Verdict::Holds, Verdict::Holds) => None,
        _ => Some(FitDiagnostic {
            code: C002,
            message: "clearance meets the required range for some values and not others".to_owned(),
        }),
    })
}

/// Evaluates a fit declaration (language.md 3.7) into its computed members.
///
/// `first` is the value of the `on` path for class fits (`H7/p6`, `p6`, `H7`)
/// or of the first path (the hole) for `a / b`; `second` is the shaft path of
/// `a / b` and must be `None` otherwise. `range` is the evaluated `[lo .. hi]`.
pub fn analyze(
    spec: &FitSpec,
    first: &Toleranced,
    second: Option<&Toleranced>,
    range: Option<(&Toleranced, &Toleranced)>,
) -> Result<FitResult, FitFailure> {
    let dim = first.dim().clone();
    let mut diagnostics = Vec::new();
    let (hole, shaft) = match (spec, second) {
        (FitSpec::Pair(a, b), None) => {
            let (sa, ra) = class_side(&dim, first.nominal(), &a.name)?;
            let (sb, rb) = class_side(&dim, first.nominal(), &b.name)?;
            if !untoleranced(first) && !matches_class(first, &ra)? && !matches_class(first, &rb)? {
                warn!(fit = %a.name, "F003 fitted parameter disagrees with its class");
                diagnostics.push(FitDiagnostic {
                    code: F003,
                    message: format!(
                        "the fitted parameter's own tolerance matches neither `{}` nor `{}`",
                        a.name, b.name
                    ),
                });
            }
            order(sa, sb)
        }
        (FitSpec::Class(c), None) => {
            let (mine, _) = class_side(&dim, first.nominal(), &c.name)?;
            let other = FitSide {
                value: first.clone(),
                role: info(mine.role.role.mate(), RoleSource::FitSide),
            };
            order(mine, other)
        }
        (FitSpec::Paths(_, _), Some(s)) => {
            s.dim()
                .check_same_dimension(&dim)
                .map_err(|_| ToleranceError::DimensionMismatch {
                    left: format!("{dim:?}"),
                    right: format!("{:?}", s.dim()),
                })?;
            (
                FitSide {
                    value: first.clone(),
                    role: info(Role::Hole, RoleSource::FitSide),
                },
                FitSide {
                    value: s.clone(),
                    role: info(Role::Shaft, RoleSource::FitSide),
                },
            )
        }
        _ => return Err(FitFailure::OperandMismatch),
    };
    let clearance = clearance_of(&hole.value, &shaft.value)?;
    let fit_type = fit_type(&clearance)?;
    if let Some((lo, hi)) = range {
        diagnostics.extend(check_range(&clearance, lo, hi)?);
    }
    debug!(?fit_type, n_diag = diagnostics.len(), "fit analysed");
    Ok(FitResult {
        hole,
        shaft,
        clearance,
        fit_type,
        diagnostics,
    })
}

/// Orders two sides as (hole, shaft) by their roles.
fn order(a: FitSide, b: FitSide) -> (FitSide, FitSide) {
    if a.role.role == Role::Hole {
        (a, b)
    } else {
        (b, a)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadetta_syntax::ast::{Ident, Span};
    use cadetta_units::Dimension;

    fn len() -> Dim {
        Dim::new(Dimension::from_si([1, 0, 0, 0, 0, 0, 0]), 0)
    }

    fn mm(n: i64) -> Magnitude {
        Magnitude::new(Rational::new(n, 1000).unwrap(), 0)
    }

    fn mm_frac(n: i64, d: i64) -> Magnitude {
        Magnitude::new(Rational::new(n, 1000 * d).unwrap(), 0)
    }

    fn plain(n: i64) -> Toleranced {
        Toleranced::symmetric(len(), mm(n), (len(), mm(0))).unwrap()
    }

    fn toleranced(n: i64, lo_nm: i64, hi_nm: i64) -> Toleranced {
        Toleranced::from_deviations(
            len(),
            mm(n),
            (len(), nm_magnitude(lo_nm).unwrap()),
            (len(), nm_magnitude(hi_nm).unwrap()),
        )
        .unwrap()
    }

    fn ident(s: &str) -> Ident {
        Ident {
            name: s.to_owned(),
            span: Span::default(),
        }
    }

    fn range(lo: i64, hi: i64) -> (Toleranced, Toleranced) {
        // Hundredths of a millimetre.
        let m = |n: i64| Toleranced::symmetric(len(), mm_frac(n, 100), (len(), mm(0))).unwrap();
        (m(lo), m(hi))
    }

    // frob:tests crates/cadetta-sema/src/fits.rs::analyze kind=unit
    // frob:ticket 01M4SPWQ0WTDXX64VS40QGESR5
    #[test]
    fn pair_has_members_and_interference_type() {
        let spec = FitSpec::Pair(ident("H7"), ident("p6"));
        let r = analyze(&spec, &plain(20), None, None).unwrap();
        // H7 +0/+21 um, p6 +22/+35 um: clearance -35..-1 um.
        assert_eq!(r.fit_type, FitType::Interference);
        assert_eq!(r.fit_type.name(), "interference");
        assert_eq!(r.clearance.min().unwrap(), nm_magnitude(-35_000).unwrap());
        assert_eq!(r.clearance.max().unwrap(), nm_magnitude(-1_000).unwrap());
        assert!(r.diagnostics.is_empty());
    }

    // frob:tests crates/cadetta-sema/src/fits.rs::analyze kind=unit
    // frob:ticket 01M4SPWQ0WTDXX64VS40QGESR5
    #[test]
    fn class_letter_case_picks_the_side() {
        let bore = toleranced(20, 0, 21_000);
        let shaft_class = analyze(&FitSpec::Class(ident("p6")), &bore, None, None).unwrap();
        assert_eq!(shaft_class.shaft.role.role, Role::Shaft);
        assert_eq!(shaft_class.hole.role.role, Role::Hole);
        assert_eq!(shaft_class.hole.value, bore);
        assert_eq!(shaft_class.hole.role.source, RoleSource::FitSide);
        assert_eq!(shaft_class.shaft.role.source, RoleSource::IsoClass);

        let shaft = toleranced(20, 22_000, 35_000);
        let hole_class = analyze(&FitSpec::Class(ident("H7")), &shaft, None, None).unwrap();
        assert_eq!(hole_class.shaft.value, shaft);
        assert_eq!(hole_class.hole.role.role, Role::Hole);
        assert_eq!(hole_class.hole.role.source, RoleSource::IsoClass);
        assert_eq!(hole_class.fit_type, FitType::Interference);
    }

    // frob:tests crates/cadetta-sema/src/fits.rs::analyze kind=unit
    // frob:ticket 01M4SPWQ0WTDXX64VS40QGESR5
    #[test]
    fn path_pair_uses_both_own_tolerances() {
        let hole = toleranced(10, 0, 15_000);
        let od = toleranced(10, -20_000, -5_000);
        let r = analyze(&FitSpec::Paths(path(), path()), &hole, Some(&od), None).unwrap();
        assert_eq!(r.hole.value, hole);
        assert_eq!(r.shaft.value, od);
        assert_eq!(r.fit_type, FitType::Clearance);
        assert_eq!(r.clearance.max().unwrap(), nm_magnitude(35_000).unwrap());
    }

    fn path() -> cadetta_syntax::ast::Path {
        cadetta_syntax::ast::Path {
            segments: vec![ident("x")],
            seps: vec![],
            span: Span::default(),
        }
    }

    // frob:tests crates/cadetta-sema/src/fits.rs::analyze kind=unit
    // frob:ticket 01M4SPWQ0WTDXX64VS40QGESR5
    #[test]
    fn transition_when_clearance_spans_zero() {
        let hole = toleranced(10, 0, 15_000);
        let od = toleranced(10, -5_000, 5_000);
        let r = analyze(&FitSpec::Paths(path(), path()), &hole, Some(&od), None).unwrap();
        assert_eq!(r.fit_type, FitType::Transition);
    }

    // frob:tests crates/cadetta-sema/src/fits.rs::analyze kind=unit
    // frob:ticket 01M4SPWQ0WTDXX64VS40QGESR5
    #[test]
    fn required_range_outside_is_f004_partial_is_c002() {
        let spec = FitSpec::Pair(ident("H7"), ident("p6"));
        // Clearance is -0.035..-0.001 mm.
        let (lo, hi) = range(-4, -1);
        let partial = analyze(&spec, &plain(20), None, Some((&lo, &hi))).unwrap();
        assert_eq!(partial.diagnostics[0].code, C002);

        let (lo, hi) = range(1, 4);
        let outside = analyze(&spec, &plain(20), None, Some((&lo, &hi))).unwrap();
        assert_eq!(outside.diagnostics[0].code, F004);

        let (lo, hi) = range(-10, 0);
        let inside = analyze(&spec, &plain(20), None, Some((&lo, &hi))).unwrap();
        assert!(inside.diagnostics.is_empty());
    }

    // frob:tests crates/cadetta-sema/src/fits.rs::analyze kind=unit
    // frob:ticket 01M4SPWQ0WTDXX64VS40QGESR5
    #[test]
    fn disagreeing_fitted_tolerance_is_f003() {
        let spec = FitSpec::Pair(ident("H7"), ident("p6"));
        let bad = toleranced(20, 0, 30_000);
        let r = analyze(&spec, &bad, None, None).unwrap();
        assert_eq!(r.diagnostics[0].code, F003);
        let good = toleranced(20, 0, 21_000);
        assert!(
            analyze(&spec, &good, None, None)
                .unwrap()
                .diagnostics
                .is_empty()
        );
    }

    // frob:tests crates/cadetta-sema/src/fits.rs::analyze kind=unit
    // frob:ticket 01M4SPWQ0WTDXX64VS40QGESR5
    #[test]
    fn unknown_class_and_size_are_f001_f002() {
        let e = analyze(&FitSpec::Class(ident("Z9")), &plain(20), None, None).unwrap_err();
        assert_eq!(e.code(), Some("F001"));
        let e = analyze(&FitSpec::Class(ident("p6")), &plain(600), None, None).unwrap_err();
        assert_eq!(e.code(), Some("F002"));
        let frac = Toleranced::symmetric(
            len(),
            Magnitude::new(Rational::new(1_i64, 3_000_000_000_i64).unwrap(), 0),
            (len(), mm(0)),
        )
        .unwrap();
        let e = analyze(&FitSpec::Class(ident("p6")), &frac, None, None).unwrap_err();
        assert_eq!(e, FitFailure::NominalNotNanometres);
    }

    // frob:tests FitFailure.code
    #[test]
    fn failure_codes_name_the_lint_rule() {
        assert_eq!(FitFailure::NominalNotNanometres.code(), Some("F002"));
        let iso = FitFailure::Iso(FitError::UnknownClass { class: "Z9".to_owned() });
        assert_eq!(iso.code(), Some("F001"));
        assert_eq!(FitFailure::OperandMismatch.code(), None);
    }
}
