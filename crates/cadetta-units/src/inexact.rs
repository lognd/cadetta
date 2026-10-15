// frob:ticket 01M4SPWNVX14PTZ4B19436P66Q
//! Inexact values and rounding (quantities.md 1, 4.3, 5.5).
//!
//! A [`Value`] is either an exact [`Magnitude`] or an [`InexactValue`]: an
//! IEEE-754 binary64 that names the root operation that made it inexact, so
//! Q006 can be reported once at the root. Roots, trigonometry and mixed-`pi^k`
//! sums are exact only where the result is rational in `pi` (or rational);
//! everything else is marked. `round_to`, `ceil_to` and `floor_to` restore
//! exactness, and report Q010 when an inexact value sits on a boundary.

use num_bigint::{BigInt, Sign};
use num_integer::Integer;
use num_traits::{One, Signed, ToPrimitive};
use thiserror::Error;
use tracing::{debug, trace};

use crate::dimension::{Dim, DimensionError};
use crate::kind::Rounding;
use crate::rational::{Magnitude, MagnitudeError, Rational, Sum};
use crate::tolerance::{ToleranceError, Toleranced};

/// Relative distance to a tie or step boundary that counts as ambiguous.
pub const AMBIGUITY_REL_TOL: f64 = 1e-9;

/// The operation that first made a value inexact (the Q006 root).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RootOp {
    /// A root with a non-rational result.
    Sqrt,
    /// A cube root with a non-rational result.
    Cbrt,
    /// Sine at an angle that is not an exact special case.
    Sin,
    /// Cosine at an angle that is not an exact special case.
    Cos,
    /// Tangent at an angle that is not an exact special case.
    Tan,
    /// Two-argument arctangent that is not an exact special case.
    Atan2,
    /// Addition across different nonzero powers of pi.
    MixedPiSum,
    /// `floor`, `ceil` or `round` of a value that carries a pi factor.
    PiRound,
}

impl RootOp {
    /// Short name used in diagnostics (Q006 messages).
    pub fn name(self) -> &'static str {
        match self {
            Self::Sqrt => "sqrt",
            Self::Cbrt => "cbrt",
            Self::Sin => "sin",
            Self::Cos => "cos",
            Self::Tan => "tan",
            Self::Atan2 => "atan2",
            Self::MixedPiSum => "sum across pi exponents",
            Self::PiRound => "rounding of a pi-carrying value",
        }
    }
}

/// A binary64 explicitly marked inexact, naming its root operation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InexactValue {
    /// The SI-base value as binary64.
    pub value: f64,
    /// The root operation that introduced the inexactness.
    pub root: RootOp,
}

/// An exact magnitude or a marked-inexact binary64 (quantities.md 1).
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// Exact `r * pi^k`.
    Exact(Magnitude),
    /// Marked inexact.
    Inexact(InexactValue),
}

/// Errors from inexact arithmetic and rounding, returned as values.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum InexactError {
    /// The argument lies outside the function's domain.
    #[error("argument outside the domain of {0}")]
    Domain(&'static str),
    /// Operands of `atan2` or a rounding step differ in dimension.
    #[error(transparent)]
    Dimension(#[from] DimensionError),
    /// The rounding step is zero.
    #[error("rounding step is zero")]
    ZeroStep,
    /// The rounded value or step is not a finite number.
    #[error("non-finite value in rounding")]
    NonFinite,
    /// Q010 ambiguous-rounding: an inexact value lies on a tie or boundary.
    #[error("Q010 ambiguous-rounding: inexact value within 1e-9 of a tie or step boundary")]
    AmbiguousRounding,
    /// An underlying exact-arithmetic failure.
    #[error(transparent)]
    Magnitude(#[from] MagnitudeError),
    /// Rebuilding a tolerance after rounding failed.
    #[error(transparent)]
    Tolerance(#[from] ToleranceError),
}

impl InexactError {
    /// The spec diagnostic code for this error, where one exists.
    pub fn code(&self) -> Option<&'static str> {
        match self {
            Self::AmbiguousRounding => Some("Q010"),
            _ => None,
        }
    }
}

fn mag(n: i64, d: i64, k: i32) -> Value {
    Value::Exact(Magnitude::new(
        Rational::new(n, d).expect("nonzero denominator"),
        k,
    ))
}

fn inexact(value: f64, root: RootOp) -> Value {
    debug!(value, root = root.name(), "inexact: value marked inexact");
    Value::Inexact(InexactValue { value, root })
}

impl Value {
    /// Nearest binary64 (exact values are converted, never stored so).
    pub fn to_f64(&self) -> f64 {
        match self {
            Self::Exact(m) => m.to_f64(),
            Self::Inexact(i) => i.value,
        }
    }

    /// True when the value is marked inexact.
    pub fn is_inexact(&self) -> bool {
        matches!(self, Self::Inexact(_))
    }

    /// The root operation if inexact.
    pub fn root(&self) -> Option<RootOp> {
        match self {
            Self::Exact(_) => None,
            Self::Inexact(i) => Some(i.root),
        }
    }

    /// Lift the result of [`Magnitude::add`]; mixed `pi^k` is marked inexact.
    pub fn from_sum(sum: Sum) -> Self {
        match sum {
            Sum::Exact(m) => Self::Exact(m),
            Sum::Inexact(f) => inexact(f, RootOp::MixedPiSum),
        }
    }

    /// Combine with `exact` when both are exact, else with `f` on binary64.
    ///
    /// The root of the first inexact operand is kept, so Q006 is reported once.
    fn combine(
        &self,
        o: &Self,
        exact: impl Fn(&Magnitude, &Magnitude) -> Value,
        f: impl Fn(f64, f64) -> f64,
    ) -> Value {
        match (self, o) {
            (Self::Exact(a), Self::Exact(b)) => exact(a, b),
            _ => {
                let root = self
                    .root()
                    .or_else(|| o.root())
                    .expect("an operand is inexact");
                trace!(
                    root = root.name(),
                    "inexact: propagating through arithmetic"
                );
                Value::Inexact(InexactValue {
                    value: f(self.to_f64(), o.to_f64()),
                    root,
                })
            }
        }
    }

    /// Sum; inexact if either operand is, or across different `pi^k`.
    pub fn add(&self, o: &Self) -> Value {
        self.combine(o, |a, b| Value::from_sum(a.add(b)), |a, b| a + b)
    }

    /// Difference; same exactness rule as [`Value::add`].
    pub fn sub(&self, o: &Self) -> Value {
        self.combine(o, |a, b| Value::from_sum(a.sub(b)), |a, b| a - b)
    }

    /// Product; inexact if either operand is.
    pub fn mul(&self, o: &Self) -> Value {
        self.combine(o, |a, b| Value::Exact(a.mul(b)), |a, b| a * b)
    }

    /// Quotient; inexact if either operand is; division by zero is an error.
    pub fn div(&self, o: &Self) -> Result<Value, InexactError> {
        if o.to_f64() == 0.0 {
            return Err(MagnitudeError::ZeroDenominator.into());
        }
        let v = self.combine(
            o,
            |a, b| Value::Exact(a.div(b).expect("divisor checked nonzero")),
            |a, b| a / b,
        );
        Ok(v)
    }

    /// Negation, preserving exactness and root.
    pub fn neg(&self) -> Value {
        match self {
            Self::Exact(m) => Self::Exact(m.neg()),
            Self::Inexact(i) => Self::Inexact(InexactValue {
                value: -i.value,
                root: i.root,
            }),
        }
    }

    /// Square root: exact only for a perfect-square rational with even `pi` exponent.
    pub fn sqrt(&self) -> Result<Value, InexactError> {
        match self {
            Self::Exact(m) => {
                if m.rational().numer().is_negative() {
                    return Err(InexactError::Domain("sqrt"));
                }
                let (n, d) = (m.rational().numer(), m.rational().denom());
                let (rn, rd) = (n.sqrt(), d.sqrt());
                if m.pi_exp() % 2 == 0 && &(&rn * &rn) == n && &(&rd * &rd) == d {
                    let r = Rational::new(rn, rd)?;
                    trace!("inexact: sqrt is exact");
                    return Ok(Self::Exact(Magnitude::new(r, m.pi_exp() / 2)));
                }
                Ok(inexact(m.to_f64().sqrt(), RootOp::Sqrt))
            }
            Self::Inexact(i) => {
                if i.value < 0.0 {
                    return Err(InexactError::Domain("sqrt"));
                }
                Ok(Self::Inexact(InexactValue {
                    value: i.value.sqrt(),
                    root: i.root,
                }))
            }
        }
    }

    /// Cube root: exact only for a perfect-cube rational with `pi` exponent divisible by 3.
    // frob:ticket 01M4YMKHAW1T9QY9FWJNQ0HFFK
    pub fn cbrt(&self) -> Value {
        match self {
            Self::Exact(m) => {
                let (n, d) = (m.rational().numer(), m.rational().denom());
                let (rn, rd) = (n.cbrt(), d.cbrt());
                if m.pi_exp() % 3 == 0
                    && &rn.pow(3) == n
                    && &rd.pow(3) == d
                    && let Ok(r) = Rational::new(rn, rd)
                {
                    trace!("inexact: cbrt is exact");
                    return Self::Exact(Magnitude::new(r, m.pi_exp() / 3));
                }
                inexact(m.to_f64().cbrt(), RootOp::Cbrt)
            }
            Self::Inexact(i) => Self::Inexact(InexactValue {
                value: i.value.cbrt(),
                root: i.root,
            }),
        }
    }

    /// Sine of an angle in radians; exact at multiples of 90 and 30 degrees.
    pub fn sin(&self) -> Value {
        self.trig(RootOp::Sin)
    }

    /// Cosine of an angle in radians; exact at multiples of 90 and 60 degrees.
    pub fn cos(&self) -> Value {
        self.trig(RootOp::Cos)
    }

    /// Tangent of an angle in radians; exact at multiples of 45 degrees.
    ///
    /// A pole (odd multiple of 90 degrees) is a domain error.
    pub fn tan(&self) -> Result<Value, InexactError> {
        if let Self::Exact(m) = self
            && let Some(p) = eighths_of_turn(m)
        {
            {
                return match p % 4 {
                    0 => Ok(mag(0, 1, 0)),
                    2 => Err(InexactError::Domain("tan")),
                    1 => Ok(mag(1, 1, 0)),
                    _ => Ok(mag(-1, 1, 0)),
                };
            }
        }
        Ok(self.map_f64(f64::tan, RootOp::Tan))
    }

    fn trig(&self, op: RootOp) -> Value {
        if let Self::Exact(m) = self
            && let Some(n) = twelfths_of_turn(m)
        {
            {
                // Angle is n * 30 degrees, n in 0..12.
                let sin: [Option<(i64, i64)>; 12] = [
                    Some((0, 1)),
                    Some((1, 2)),
                    None,
                    Some((1, 1)),
                    None,
                    Some((1, 2)),
                    Some((0, 1)),
                    Some((-1, 2)),
                    None,
                    Some((-1, 1)),
                    None,
                    Some((-1, 2)),
                ];
                let idx = if op == RootOp::Sin { n } else { (n + 3) % 12 };
                if let Some((a, b)) = sin[idx as usize] {
                    trace!(op = op.name(), "inexact: exact special angle");
                    return mag(a, b, 0);
                }
            }
        }
        let f: fn(f64) -> f64 = if op == RootOp::Sin {
            f64::sin
        } else {
            f64::cos
        };
        self.map_f64(f, op)
    }

    fn map_f64(&self, f: impl Fn(f64) -> f64, op: RootOp) -> Value {
        match self {
            Self::Exact(m) => inexact(f(m.to_f64()), op),
            Self::Inexact(i) => Self::Inexact(InexactValue {
                value: f(i.value),
                root: i.root,
            }),
        }
    }
}

/// `m` as an integer count of `turn/12` (30 degrees), reduced to `0..12`.
fn twelfths_of_turn(m: &Magnitude) -> Option<i64> {
    turn_fraction(m, 6)
}

/// `m` as an integer count of 45 degrees, reduced to `0..8`.
fn eighths_of_turn(m: &Magnitude) -> Option<i64> {
    turn_fraction(m, 4)
}

/// `m = r * pi` with `r * per_pi` an integer; returns that integer mod `2 * per_pi`.
fn turn_fraction(m: &Magnitude, per_pi: i64) -> Option<i64> {
    if m.is_zero() {
        return Some(0);
    }
    if m.pi_exp() != 1 {
        return None;
    }
    let scaled = m.rational().mul(&Rational::from_int(per_pi));
    if !scaled.denom().is_one() {
        return None;
    }
    scaled.numer().mod_floor(&BigInt::from(2 * per_pi)).to_i64()
}

/// `atan2(y, x)` of two same-dimension values; exact where the result is rational in pi.
///
/// Exact when `y == 0`, `x == 0` or `|y| == |x|`; `atan2(0, 0)` is a domain error.
pub fn atan2(y: &Value, y_dim: &Dim, x: &Value, x_dim: &Dim) -> Result<Value, InexactError> {
    y_dim.check_same_dimension(x_dim)?;
    if let (Value::Exact(ym), Value::Exact(xm)) = (y, x) {
        // Multiples of pi/4 as (numerator of 1/4 turns of pi); k = 1.
        let sy = ym.rational().numer().sign();
        let sx = xm.rational().numer().sign();
        if ym.pi_exp() == xm.pi_exp() || ym.is_zero() || xm.is_zero() {
            let quarters = match (sy, sx) {
                (Sign::NoSign, Sign::NoSign) => return Err(InexactError::Domain("atan2")),
                (Sign::NoSign, Sign::Plus) => Some(0),
                (Sign::NoSign, _) => Some(4),
                (Sign::Plus, Sign::NoSign) => Some(2),
                (_, Sign::NoSign) => Some(-2),
                _ if ym.rational().numer().abs() == xm.rational().numer().abs()
                    && ym.rational().denom() == xm.rational().denom()
                    && ym.pi_exp() == xm.pi_exp() =>
                {
                    Some(match (sy, sx) {
                        (Sign::Plus, Sign::Plus) => 1,
                        (Sign::Plus, _) => 3,
                        (_, Sign::Plus) => -1,
                        _ => -3,
                    })
                }
                _ => None,
            };
            if let Some(q) = quarters {
                trace!(q, "inexact: atan2 exact special case");
                return Ok(mag(q, 4, 1));
            }
        }
    }
    let r = y.to_f64().atan2(x.to_f64());
    let root = y.root().or_else(|| x.root()).unwrap_or(RootOp::Atan2);
    Ok(inexact(r, root))
}

/// Round `x` to a multiple of `step` (quantities.md 5.5); the result is exact.
///
/// `mode` is [`Rounding::Round`] (ties away from zero), [`Rounding::Ceil`] or
/// [`Rounding::Floor`]. Dimensions must match; an inexact `x` within 1e-9
/// (relative) of a tie (round) or step boundary (ceil, floor) is Q010.
pub fn round_value(
    x: &Value,
    x_dim: &Dim,
    step: &Magnitude,
    step_dim: &Dim,
    mode: Rounding,
) -> Result<Magnitude, InexactError> {
    x_dim.check_same_dimension(step_dim)?;
    if step.is_zero() {
        return Err(InexactError::ZeroStep);
    }
    let n = match x {
        Value::Exact(m) => {
            let q = m.div(step)?;
            if q.pi_exp() == 0 || q.is_zero() {
                multiple_exact(q.rational(), mode)
            } else {
                // An irrational quotient (differing pi exponents) is inexact.
                multiple_f64(q.to_f64(), mode, true)?
            }
        }
        Value::Inexact(i) => multiple_f64(i.value / step.to_f64(), mode, true)?,
    };
    debug!(?mode, %n, "inexact: rounded to exact multiple of step");
    Ok(step.mul(&Magnitude::new(Rational::from_int(n), 0)))
}

/// Round the nominal of a toleranced value, keeping its deviations (5.5).
pub fn round_toleranced(
    x: &Toleranced,
    step: &Magnitude,
    step_dim: &Dim,
    mode: Rounding,
) -> Result<Toleranced, InexactError> {
    let nominal = round_value(
        &Value::Exact(x.nominal().clone()),
        x.dim(),
        step,
        step_dim,
        mode,
    )?;
    Ok(Toleranced::from_deviations(
        x.dim().clone(),
        nominal,
        (x.dim().clone(), x.lower().clone()),
        (x.dim().clone(), x.upper().clone()),
    )?)
}

/// `round_to(x, step)`: nearest multiple, ties away from zero.
pub fn round_to(
    x: &Value,
    x_dim: &Dim,
    step: &Magnitude,
    step_dim: &Dim,
) -> Result<Magnitude, InexactError> {
    round_value(x, x_dim, step, step_dim, Rounding::Round)
}

/// `ceil_to(x, step)`: smallest multiple not below `x`.
pub fn ceil_to(
    x: &Value,
    x_dim: &Dim,
    step: &Magnitude,
    step_dim: &Dim,
) -> Result<Magnitude, InexactError> {
    round_value(x, x_dim, step, step_dim, Rounding::Ceil)
}

/// `floor_to(x, step)`: largest multiple not above `x`.
pub fn floor_to(
    x: &Value,
    x_dim: &Dim,
    step: &Magnitude,
    step_dim: &Dim,
) -> Result<Magnitude, InexactError> {
    round_value(x, x_dim, step, step_dim, Rounding::Floor)
}

/// Integer multiple of an exact rational quotient; ties away from zero.
fn multiple_exact(q: &Rational, mode: Rounding) -> BigInt {
    let (n, d) = (q.numer(), q.denom());
    match mode {
        Rounding::Floor => n.div_floor(d),
        Rounding::Ceil => -((-n).div_floor(d)),
        Rounding::Round => {
            // floor(|q| + 1/2) = floor((2|n| + d) / 2d), then restore the sign.
            let two = BigInt::from(2);
            let r = (&two * n.abs() + d).div_floor(&(&two * d));
            if n.is_negative() { -r } else { r }
        }
    }
}

/// Integer multiple of an inexact quotient, or Q010 near a tie or boundary.
fn multiple_f64(q: f64, mode: Rounding, check: bool) -> Result<BigInt, InexactError> {
    if !q.is_finite() {
        return Err(InexactError::NonFinite);
    }
    let boundary = match mode {
        Rounding::Round => (q.abs() - 0.5).round() + 0.5,
        _ => q.abs().round(),
    };
    let tol = AMBIGUITY_REL_TOL * boundary.max(1.0);
    if check && (q.abs() - boundary).abs() <= tol {
        debug!(q, "inexact: Q010 ambiguous rounding");
        return Err(InexactError::AmbiguousRounding);
    }
    let r = match mode {
        Rounding::Floor => q.floor(),
        Rounding::Ceil => q.ceil(),
        // Away from zero on ties is moot here: ties are rejected above.
        Rounding::Round => q.round(),
    };
    Ok(BigInt::from(r as i128))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dimension::Dimension;

    fn len() -> Dim {
        Dim::new(Dimension::from_si([1, 0, 0, 0, 0, 0, 0]), 0)
    }
    fn area() -> Dim {
        len().powi(2)
    }
    fn q(n: i64, d: i64) -> Rational {
        Rational::new(n, d).unwrap()
    }
    fn m(n: i64, d: i64) -> Magnitude {
        Magnitude::new(q(n, d), 0)
    }
    fn ex(n: i64, d: i64) -> Value {
        Value::Exact(m(n, d))
    }
    fn deg(n: i64) -> Value {
        Value::Exact(Magnitude::new(q(n, 180), 1))
    }
    fn inx(v: f64) -> Value {
        Value::Inexact(InexactValue {
            value: v,
            root: RootOp::Sqrt,
        })
    }

    // frob:tests crates/cadetta-units/src/inexact.rs::Value.sqrt kind=unit
    // frob:tests crates/cadetta-units/src/inexact.rs::Value.cbrt kind=unit
    #[test]
    fn cbrt_is_exact_for_cubes_and_marked_inexact_otherwise() {
        let c = Value::Exact(Magnitude::new(Rational::new(-8, 27).unwrap(), 0)).cbrt();
        assert_eq!(
            c,
            Value::Exact(Magnitude::new(Rational::new(-2, 3).unwrap(), 0))
        );
        let two = Value::Exact(Magnitude::new(Rational::new(2, 1).unwrap(), 0)).cbrt();
        assert_eq!(two.root(), Some(RootOp::Cbrt));
        assert!((two.to_f64() - 2f64.cbrt()).abs() < 1e-15);
        assert_eq!(two.cbrt().root(), Some(RootOp::Cbrt));
    }

    #[test]
    fn sqrt_of_two_square_mm_is_marked_inexact_with_root() {
        let two_mm2 = ex(2, 1_000_000);
        let r = two_mm2.sqrt().unwrap();
        assert_eq!(r.root(), Some(RootOp::Sqrt));
        assert!((r.to_f64() - 2f64.sqrt() * 1e-3).abs() < 1e-15);
        assert_eq!(ex(4, 1_000_000).sqrt().unwrap(), ex(1, 500));
        assert_eq!(ex(-1, 1).sqrt(), Err(InexactError::Domain("sqrt")));
    }

    // frob:tests crates/cadetta-units/src/inexact.rs::Value.add kind=unit
    #[test]
    fn inexact_plus_exact_is_inexact_and_keeps_root() {
        let s = inx(1.5).add(&ex(1, 1));
        assert_eq!(s.root(), Some(RootOp::Sqrt));
        assert_eq!(s.to_f64(), 2.5);
        assert!(ex(1, 1).sub(&inx(1.0)).is_inexact());
        assert!(inx(2.0).mul(&ex(3, 1)).is_inexact());
        assert!(inx(2.0).div(&ex(2, 1)).unwrap().is_inexact());
        assert!(!ex(1, 2).add(&ex(1, 3)).is_inexact());
        // Mixed pi exponents are inexact with their own root.
        let one = Value::Exact(m(1, 1));
        let pi = Value::Exact(Magnitude::new(q(1, 1), 1));
        assert_eq!(one.add(&pi).root(), Some(RootOp::MixedPiSum));
        assert_eq!(
            ex(1, 1).div(&ex(0, 1)),
            Err(MagnitudeError::ZeroDenominator.into())
        );
    }

    // frob:tests crates/cadetta-units/src/inexact.rs::Value.sin kind=unit
    #[test]
    fn trig_is_exact_at_special_angles() {
        for (d, s, c) in [
            (0, (0, 1), (1, 1)),
            (30, (1, 2), (0, 0)),
            (90, (1, 1), (0, 1)),
            (150, (1, 2), (0, 0)),
            (180, (0, 1), (-1, 1)),
            (270, (-1, 1), (0, 1)),
            (330, (-1, 2), (0, 0)),
            (-90, (-1, 1), (0, 1)),
            (720, (0, 1), (1, 1)),
        ] {
            assert_eq!(deg(d).sin(), ex(s.0, s.1), "sin {d}");
            if c.1 != 0 {
                assert_eq!(deg(d).cos(), ex(c.0, c.1), "cos {d}");
            }
        }
        assert_eq!(deg(60).cos(), ex(1, 2));
        assert_eq!(deg(300).cos(), ex(1, 2));
        assert_eq!(deg(120).cos(), ex(-1, 2));
        assert_eq!(deg(45).tan().unwrap(), ex(1, 1));
        assert_eq!(deg(225).tan().unwrap(), ex(1, 1));
        assert_eq!(deg(135).tan().unwrap(), ex(-1, 1));
        assert_eq!(deg(180).tan().unwrap(), ex(0, 1));
        assert_eq!(deg(90).tan(), Err(InexactError::Domain("tan")));
        // Non-special angles are inexact, as is any non-pi argument.
        assert_eq!(deg(45).sin().root(), Some(RootOp::Sin));
        assert_eq!(deg(60).sin().root(), Some(RootOp::Sin));
        assert_eq!(deg(30).tan().unwrap().root(), Some(RootOp::Tan));
        assert_eq!(ex(1, 1).cos().root(), Some(RootOp::Cos));
    }

    // frob:tests crates/cadetta-units/src/inexact.rs::atan2 kind=unit
    #[test]
    fn atan2_special_cases_are_exact() {
        let a = |y: Value, x: Value| atan2(&y, &len(), &x, &len()).unwrap();
        assert_eq!(a(ex(3, 1), ex(3, 1)), deg(45));
        assert_eq!(a(ex(3, 1), ex(-3, 1)), deg(135));
        assert_eq!(a(ex(-3, 1), ex(3, 1)), deg(-45));
        assert_eq!(a(ex(-3, 1), ex(-3, 1)), deg(-135));
        assert_eq!(a(ex(0, 1), ex(2, 1)), deg(0));
        assert_eq!(a(ex(0, 1), ex(-2, 1)), deg(180));
        assert_eq!(a(ex(2, 1), ex(0, 1)), deg(90));
        assert_eq!(a(ex(-2, 1), ex(0, 1)), deg(-90));
        let r = a(ex(1, 1), ex(2, 1));
        assert_eq!(r.root(), Some(RootOp::Atan2));
        assert!((r.to_f64() - 0.5f64.atan()).abs() < 1e-15);
        assert_eq!(
            atan2(&ex(0, 1), &len(), &ex(0, 1), &len()),
            Err(InexactError::Domain("atan2"))
        );
        assert!(matches!(
            atan2(&ex(1, 1), &len(), &ex(1, 1), &area()),
            Err(InexactError::Dimension(_))
        ));
    }

    // frob:tests crates/cadetta-units/src/inexact.rs::round_to kind=unit
    #[test]
    fn rounding_exact_values_is_exact_ties_away_from_zero() {
        let step = m(1, 2);
        let r = |x: Value| round_to(&x, &len(), &step, &len()).unwrap();
        assert_eq!(r(ex(3, 4)), m(1, 1));
        assert_eq!(r(ex(-3, 4)), m(-1, 1));
        assert_eq!(r(ex(5, 8)), m(1, 2));
        assert_eq!(r(ex(7, 10)), m(1, 2));
        let one = m(1, 1);
        let rr = |x: Value| round_to(&x, &len(), &one, &len()).unwrap();
        assert_eq!(rr(ex(5, 2)), m(3, 1));
        assert_eq!(rr(ex(-5, 2)), m(-3, 1));
        assert_eq!(rr(ex(3, 2)), m(2, 1));
        assert_eq!(rr(ex(1, 2)), m(1, 1));
        assert_eq!(rr(ex(-1, 2)), m(-1, 1));
        assert_eq!(rr(ex(1, 3)), m(0, 1));
    }

    // frob:tests crates/cadetta-units/src/inexact.rs::ceil_to kind=unit
    #[test]
    fn ceil_and_floor_to_step() {
        let step = m(1, 10);
        let c = |x: Value| ceil_to(&x, &len(), &step, &len()).unwrap();
        let f = |x: Value| floor_to(&x, &len(), &step, &len()).unwrap();
        assert_eq!(c(ex(23, 100)), m(3, 10));
        assert_eq!(f(ex(23, 100)), m(2, 10));
        assert_eq!(c(ex(-23, 100)), m(-2, 10));
        assert_eq!(f(ex(-23, 100)), m(-3, 10));
        assert_eq!(c(ex(3, 10)), m(3, 10));
        assert_eq!(f(ex(3, 10)), m(3, 10));
    }

    // frob:tests crates/cadetta-units/src/inexact.rs::round_toleranced kind=unit
    #[test]
    fn rounding_a_toleranced_value_keeps_deviations() {
        let t = Toleranced::from_deviations(
            len(),
            m(123, 100),
            (len(), m(-1, 100)),
            (len(), m(3, 100)),
        )
        .unwrap();
        let r = round_toleranced(&t, &m(1, 10), &len(), Rounding::Round).unwrap();
        assert_eq!(r.nominal(), &m(12, 10));
        assert_eq!(r.lower(), t.lower());
        assert_eq!(r.upper(), t.upper());
    }

    // frob:tests crates/cadetta-units/src/inexact.rs::round_value kind=unit
    #[test]
    fn inexact_near_tie_or_boundary_is_q010() {
        let one = m(1, 1);
        let near_tie = inx(2.5 + 1e-12);
        let e = round_to(&near_tie, &len(), &one, &len()).unwrap_err();
        assert_eq!(e, InexactError::AmbiguousRounding);
        assert_eq!(e.code(), Some("Q010"));
        let near_int = inx(3.0 - 1e-12);
        assert_eq!(
            ceil_to(&near_int, &len(), &one, &len()),
            Err(InexactError::AmbiguousRounding)
        );
        assert_eq!(
            floor_to(&inx(3.0 + 1e-12), &len(), &one, &len()),
            Err(InexactError::AmbiguousRounding)
        );
        // Clear of a boundary the inexact value rounds to an exact multiple.
        assert_eq!(round_to(&inx(2.4), &len(), &one, &len()).unwrap(), m(2, 1));
        assert_eq!(round_to(&inx(2.6), &len(), &one, &len()).unwrap(), m(3, 1));
        assert_eq!(ceil_to(&inx(2.4), &len(), &one, &len()).unwrap(), m(3, 1));
        // An exact value on a boundary is fine.
        assert_eq!(ceil_to(&ex(3, 1), &len(), &one, &len()).unwrap(), m(3, 1));
    }

    // frob:tests crates/cadetta-units/src/inexact.rs::InexactError kind=unit
    #[test]
    fn rounding_step_errors_are_values() {
        assert!(matches!(
            round_to(&ex(1, 1), &len(), &m(1, 1), &area()),
            Err(InexactError::Dimension(_))
        ));
        assert_eq!(
            round_to(&ex(1, 1), &len(), &m(0, 1), &len()),
            Err(InexactError::ZeroStep)
        );
        assert_eq!(
            round_to(&inx(f64::NAN), &len(), &m(1, 1), &len()),
            Err(InexactError::NonFinite)
        );
        assert_eq!(InexactError::ZeroStep.code(), None);
    }
}
