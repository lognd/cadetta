// frob:ticket 01M4SPWNNDNT60Z38K24AEKVAE
//! Tolerances as affine forms over source variables (quantities.md 5.1, 5.4).
//!
//! A literal tolerance is a pair of signed deviations `lower <= upper` on a
//! nominal ([`Toleranced`]). Evaluation carries an [`Affine`] form
//! `center + sum(c_i * e_i) +- slack` over independent source variables
//! `e_i` in `[-1, 1]`, so repeated and cancelling terms count once. Nonlinear
//! operations fall back to interval bounds and mark the result conservative.
//! Magnitudes here are SI-base values; unit conversion is the caller's job,
//! which is why deviation units may differ from the nominal's.

use std::cmp::Ordering;
use std::collections::BTreeMap;

use thiserror::Error;
use tracing::{debug, trace};

use crate::dimension::Dim;
use crate::rational::{Magnitude, MagnitudeError, Rational, Sum};

/// Errors from building or evaluating tolerances.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum ToleranceError {
    /// The lower deviation exceeds the upper one (quantities.md 5.1).
    #[error("lower deviation exceeds upper deviation")]
    InvertedDeviations,
    /// Operands (or a deviation and its nominal) differ in dimension.
    #[error("dimension mismatch: {left} vs {right}")]
    DimensionMismatch {
        /// Rendering of the expected dimension.
        left: String,
        /// Rendering of the offending dimension.
        right: String,
    },
    /// Combining terms would add different nonzero powers of pi (inexact).
    #[error("operation would mix different pi exponents inexactly")]
    MixedPiExponent,
    /// A divisor's interval contains zero.
    #[error("divisor interval contains zero")]
    DivisorContainsZero,
    /// The argument lies outside the function's domain.
    #[error("argument outside the domain of {0}")]
    Domain(&'static str),
    /// An underlying exact-arithmetic failure.
    #[error(transparent)]
    Magnitude(#[from] MagnitudeError),
}

/// Identifier of one independent source variable (quantities.md 5.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VarId(pub u32);

fn check_dim(expected: &Dim, found: &Dim) -> Result<(), ToleranceError> {
    if expected == found {
        Ok(())
    } else {
        debug!(?expected, ?found, "tolerance: dimension mismatch");
        Err(ToleranceError::DimensionMismatch {
            left: format!("{expected:?}"),
            right: format!("{found:?}"),
        })
    }
}

fn exact(sum: Sum) -> Result<Magnitude, ToleranceError> {
    match sum {
        Sum::Exact(m) => Ok(m),
        Sum::Inexact(_) => Err(ToleranceError::MixedPiExponent),
    }
}

fn add(a: &Magnitude, b: &Magnitude) -> Result<Magnitude, ToleranceError> {
    exact(a.add(b))
}

fn sub(a: &Magnitude, b: &Magnitude) -> Result<Magnitude, ToleranceError> {
    exact(a.sub(b))
}

fn half() -> Magnitude {
    Magnitude::new(Rational::new(1, 2).expect("2 is nonzero"), 0)
}

pub(crate) fn sign(m: &Magnitude) -> Ordering {
    use num_bigint::Sign;
    match m.rational().numer().sign() {
        Sign::Minus => Ordering::Less,
        Sign::NoSign => Ordering::Equal,
        Sign::Plus => Ordering::Greater,
    }
}

/// Order two magnitudes; exact when exponents agree, else via binary64.
pub(crate) fn cmp_mag(a: &Magnitude, b: &Magnitude) -> Ordering {
    match a.sub(b) {
        Sum::Exact(d) => sign(&d),
        Sum::Inexact(f) => f.partial_cmp(&0.0).unwrap_or(Ordering::Equal),
    }
}

fn min_max(vals: &[Magnitude]) -> (Magnitude, Magnitude) {
    let mut lo = vals[0].clone();
    let mut hi = vals[0].clone();
    for v in &vals[1..] {
        if cmp_mag(v, &lo) == Ordering::Less {
            lo = v.clone();
        }
        if cmp_mag(v, &hi) == Ordering::Greater {
            hi = v.clone();
        }
    }
    (lo, hi)
}

/// A literal toleranced value: nominal plus `lower <= upper` deviations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Toleranced {
    dim: Dim,
    nominal: Magnitude,
    lower: Magnitude,
    upper: Magnitude,
}

impl Toleranced {
    /// Build from signed deviations (`10mm [+-0.05mm]` is `-0.05, +0.05`).
    ///
    /// Deviation dimensions are given separately because their units may differ.
    pub fn from_deviations(
        dim: Dim,
        nominal: Magnitude,
        lower: (Dim, Magnitude),
        upper: (Dim, Magnitude),
    ) -> Result<Self, ToleranceError> {
        check_dim(&dim, &lower.0)?;
        check_dim(&dim, &upper.0)?;
        if cmp_mag(&lower.1, &upper.1) == Ordering::Greater {
            debug!("tolerance: lower deviation exceeds upper");
            return Err(ToleranceError::InvertedDeviations);
        }
        trace!("tolerance: built from deviations");
        Ok(Self {
            dim,
            nominal,
            lower: lower.1,
            upper: upper.1,
        })
    }

    /// Build a symmetric `[+- d]` tolerance.
    pub fn symmetric(
        dim: Dim,
        nominal: Magnitude,
        dev: (Dim, Magnitude),
    ) -> Result<Self, ToleranceError> {
        let lower = (dev.0.clone(), dev.1.neg());
        Self::from_deviations(dim, nominal, lower, dev)
    }

    /// Build from an interval `lo..hi`; the nominal is the midpoint.
    pub fn from_interval(dim: Dim, lo: Magnitude, hi: Magnitude) -> Result<Self, ToleranceError> {
        if cmp_mag(&lo, &hi) == Ordering::Greater {
            return Err(ToleranceError::InvertedDeviations);
        }
        let nominal = add(&lo, &hi)?.mul(&half());
        let lower = sub(&lo, &nominal)?;
        let upper = sub(&hi, &nominal)?;
        trace!("tolerance: built from interval");
        Ok(Self {
            dim,
            nominal,
            lower,
            upper,
        })
    }

    /// The dimension of the value and its deviations.
    pub fn dim(&self) -> &Dim {
        &self.dim
    }

    /// The nominal value (`.nominal`).
    pub fn nominal(&self) -> &Magnitude {
        &self.nominal
    }

    /// The lower deviation (signed, `<=` the upper).
    pub fn lower(&self) -> &Magnitude {
        &self.lower
    }

    /// The upper deviation (signed).
    pub fn upper(&self) -> &Magnitude {
        &self.upper
    }

    /// The smallest value (`.min`).
    pub fn min(&self) -> Result<Magnitude, ToleranceError> {
        add(&self.nominal, &self.lower)
    }

    /// The largest value (`.max`).
    pub fn max(&self) -> Result<Magnitude, ToleranceError> {
        add(&self.nominal, &self.upper)
    }

    /// The midpoint of min and max (`.mid`).
    pub fn mid(&self) -> Result<Magnitude, ToleranceError> {
        Ok(add(&self.min()?, &self.max()?)?.mul(&half()))
    }

    /// Lift to an affine form over source variable `var`.
    pub fn to_affine(&self, var: VarId) -> Result<Affine, ToleranceError> {
        let mid_dev = add(&self.lower, &self.upper)?.mul(&half());
        let radius = sub(&self.upper, &self.lower)?.mul(&half());
        let mut terms = BTreeMap::new();
        if !radius.is_zero() {
            terms.insert(var, radius);
        }
        Ok(Affine {
            dim: self.dim.clone(),
            center: add(&self.nominal, &mid_dev)?,
            terms,
            discs: BTreeMap::new(),
            slack: Magnitude::new(Rational::from_int(0), 0),
            conservative: false,
        })
    }
}

/// An affine form `center + sum(c_i * e_i) +- slack` (quantities.md 5.4).
///
/// `slack` is a non-negative symmetric radius from nonlinear fallbacks; it
/// is independent of every variable, so it only ever grows.
///
/// Besides the box variables `terms` (each `e_i` in `[-1, 1]`), a form may
/// carry disc variables `discs`: a pair `(u, v)` with `u^2 + v^2 <= 1`
/// contributing `a*u + b*v` (quantities.md 5.4, a position tolerance). They
/// live in a separate map so box-only callers are untouched; a [`VarId`]
/// is either a box or a disc variable, never both (the registry allocates
/// distinct ids).
// frob:ticket 01M50C0NA43G0H8N5YSRDMWEB8
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Affine {
    dim: Dim,
    center: Magnitude,
    terms: BTreeMap<VarId, Magnitude>,
    discs: BTreeMap<VarId, (Magnitude, Magnitude)>,
    slack: Magnitude,
    conservative: bool,
}

/// A binary64 interval from an inexact nonlinear op; always conservative.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InexactInterval {
    /// Lower bound.
    pub lo: f64,
    /// Upper bound.
    pub hi: f64,
    /// Always true: the interval is an outer bound (quantities.md 5.4).
    pub conservative: bool,
}

impl Affine {
    /// An exact constant with no tolerance.
    pub fn constant(dim: Dim, value: Magnitude) -> Self {
        Self {
            dim,
            center: value,
            terms: BTreeMap::new(),
            discs: BTreeMap::new(),
            slack: Magnitude::new(Rational::from_int(0), 0),
            conservative: false,
        }
    }

    /// `center + a*u + b*v` over the disc variable `var` (`u^2 + v^2 <= 1`).
    ///
    /// A pair with both coefficients zero contributes no term.
    pub fn disc(dim: Dim, center: Magnitude, var: VarId, a: Magnitude, b: Magnitude) -> Self {
        let mut f = Self::constant(dim, center);
        if !(a.is_zero() && b.is_zero()) {
            f.discs.insert(var, (a, b));
        }
        f
    }

    /// The `(u, v)` coefficients of disc variable `var`, if it contributes.
    pub fn disc_coefficients(&self, var: VarId) -> Option<(&Magnitude, &Magnitude)> {
        self.discs.get(&var).map(|(a, b)| (a, b))
    }

    /// The dimension of the form.
    pub fn dim(&self) -> &Dim {
        &self.dim
    }

    /// The centre of the form.
    pub fn center(&self) -> &Magnitude {
        &self.center
    }

    /// The coefficient of `var`, if it contributes.
    pub fn coefficient(&self, var: VarId) -> Option<&Magnitude> {
        self.terms.get(&var)
    }

    /// True when a nonlinear fallback was used somewhere upstream.
    ///
    /// A disc whose radius `sqrt(a^2 + b^2)` is irrational also counts.
    pub fn is_conservative(&self) -> bool {
        self.conservative
            || self
                .discs
                .values()
                .any(|(a, b)| disc_norm(a, b).is_none_or(|(_, exact)| !exact))
    }

    /// True when the form has no variable terms and no slack.
    pub fn is_exact(&self) -> bool {
        self.terms.is_empty() && self.discs.is_empty() && self.slack.is_zero()
    }

    fn radius(&self) -> Result<Magnitude, ToleranceError> {
        let mut r = self.slack.clone();
        for c in self.terms.values() {
            let abs = if sign(c) == Ordering::Less {
                c.neg()
            } else {
                c.clone()
            };
            r = add(&r, &abs)?;
        }
        for (a, b) in self.discs.values() {
            let (norm, _) = disc_norm(a, b).ok_or(ToleranceError::MixedPiExponent)?;
            r = add(&r, &norm)?;
        }
        Ok(r)
    }

    /// The tightest interval `(min, max)` containing every value.
    pub fn bounds(&self) -> Result<(Magnitude, Magnitude), ToleranceError> {
        let r = self.radius()?;
        Ok((sub(&self.center, &r)?, add(&self.center, &r)?))
    }

    /// Sum; shared variables combine, so `a + a` doubles one term.
    pub fn add(&self, o: &Self) -> Result<Self, ToleranceError> {
        self.combine(o, false)
    }

    /// Difference; `a - a` cancels to zero tolerance.
    pub fn sub(&self, o: &Self) -> Result<Self, ToleranceError> {
        self.combine(o, true)
    }

    fn combine(&self, o: &Self, negate: bool) -> Result<Self, ToleranceError> {
        check_dim(&self.dim, &o.dim)?;
        let mut terms = self.terms.clone();
        for (v, c) in &o.terms {
            let c = if negate { c.neg() } else { c.clone() };
            let sum = match terms.get(v) {
                Some(mine) => add(mine, &c)?,
                None => c,
            };
            if sum.is_zero() {
                terms.remove(v);
            } else {
                terms.insert(*v, sum);
            }
        }
        let center = if negate {
            sub(&self.center, &o.center)?
        } else {
            add(&self.center, &o.center)?
        };
        let mut discs = self.discs.clone();
        for (v, (a, b)) in &o.discs {
            let (a, b) = if negate {
                (a.neg(), b.neg())
            } else {
                (a.clone(), b.clone())
            };
            let (a, b) = match discs.get(v) {
                Some((ma, mb)) => (add(ma, &a)?, add(mb, &b)?),
                None => (a, b),
            };
            if a.is_zero() && b.is_zero() {
                discs.remove(v);
            } else {
                discs.insert(*v, (a, b));
            }
        }
        trace!(negate, "tolerance: affine combine");
        Ok(Self {
            dim: self.dim.clone(),
            center,
            terms,
            discs,
            slack: add(&self.slack, &o.slack)?,
            conservative: self.conservative || o.conservative,
        })
    }

    /// Exact negation.
    pub fn neg(&self) -> Self {
        Self {
            dim: self.dim.clone(),
            center: self.center.neg(),
            terms: self.terms.iter().map(|(v, c)| (*v, c.neg())).collect(),
            discs: self
                .discs
                .iter()
                .map(|(v, (a, b))| (*v, (a.neg(), b.neg())))
                .collect(),
            slack: self.slack.clone(),
            conservative: self.conservative,
        }
    }

    /// Scale by an exact constant of dimension `dim`; stays linear and exact.
    fn scale(&self, k: &Magnitude, dim: Dim) -> Self {
        let abs = if sign(k) == Ordering::Less {
            k.neg()
        } else {
            k.clone()
        };
        Self {
            dim,
            center: self.center.mul(k),
            terms: self
                .terms
                .iter()
                .map(|(v, c)| (*v, c.mul(k)))
                .filter(|(_, c)| !c.is_zero())
                .collect(),
            discs: self
                .discs
                .iter()
                .map(|(v, (a, b))| (*v, (a.mul(k), b.mul(k))))
                .filter(|(_, (a, b))| !(a.is_zero() && b.is_zero()))
                .collect(),
            slack: self.slack.mul(&abs),
            conservative: self.conservative,
        }
    }

    /// Product. Linear when either side is exact; otherwise interval
    /// bounds, marked conservative (quantities.md 5.4; Q005 is retired).
    pub fn mul(&self, o: &Self) -> Result<Self, ToleranceError> {
        let dim = self.dim.mul(&o.dim);
        if o.is_exact() {
            return Ok(self.scale(&o.center, dim));
        }
        if self.is_exact() {
            return Ok(o.scale(&self.center, dim));
        }
        let (a, b) = self.bounds()?;
        let (c, d) = o.bounds()?;
        let (lo, hi) = min_max(&[a.mul(&c), a.mul(&d), b.mul(&c), b.mul(&d)]);
        debug!("tolerance: product falls back to interval bounds");
        Self::from_hull(dim, lo, hi)
    }

    /// Quotient. Linear for an exact divisor; otherwise interval bounds,
    /// conservative. A divisor whose interval contains zero is an error.
    pub fn div(&self, o: &Self) -> Result<Self, ToleranceError> {
        let dim = self.dim.div(&o.dim);
        let (c, d) = o.bounds()?;
        if sign(&c) != sign(&d) || sign(&c) == Ordering::Equal {
            return Err(ToleranceError::DivisorContainsZero);
        }
        let one = Magnitude::new(Rational::one(), 0);
        if o.is_exact() {
            return Ok(self.scale(&one.div(&o.center)?, dim));
        }
        let (a, b) = self.bounds()?;
        let (rc, rd) = (one.div(&c)?, one.div(&d)?);
        let (lo, hi) = min_max(&[a.mul(&rc), a.mul(&rd), b.mul(&rc), b.mul(&rd)]);
        debug!("tolerance: quotient falls back to interval bounds");
        Self::from_hull(dim, lo, hi)
    }

    pub(crate) fn from_hull(
        dim: Dim,
        lo: Magnitude,
        hi: Magnitude,
    ) -> Result<Self, ToleranceError> {
        Ok(Self {
            dim,
            center: add(&lo, &hi)?.mul(&half()),
            terms: BTreeMap::new(),
            discs: BTreeMap::new(),
            slack: sub(&hi, &lo)?.mul(&half()),
            conservative: true,
        })
    }

    fn f64_bounds(&self) -> Result<(f64, f64), ToleranceError> {
        let (lo, hi) = self.bounds()?;
        Ok((lo.to_f64(), hi.to_f64()))
    }

    /// Square root over the interval; inexact and conservative.
    pub fn sqrt(&self) -> Result<InexactInterval, ToleranceError> {
        let (lo, hi) = self.f64_bounds()?;
        if lo < 0.0 {
            return Err(ToleranceError::Domain("sqrt"));
        }
        debug!("tolerance: sqrt is inexact and conservative");
        Ok(outward(lo.sqrt(), hi.sqrt()))
    }

    /// Cube root over the interval (monotone, defined for negatives); inexact and conservative.
    // frob:ticket 01M4YMKHAW1T9QY9FWJNQ0HFFK
    pub fn cbrt(&self) -> Result<InexactInterval, ToleranceError> {
        let (lo, hi) = self.f64_bounds()?;
        debug!("tolerance: cbrt is inexact and conservative");
        Ok(outward(lo.cbrt(), hi.cbrt()))
    }

    /// Sine over the interval (radians); inexact and conservative.
    pub fn sin(&self) -> Result<InexactInterval, ToleranceError> {
        self.periodic(0.0)
    }

    /// Cosine over the interval (radians); inexact and conservative.
    pub fn cos(&self) -> Result<InexactInterval, ToleranceError> {
        self.periodic(std::f64::consts::FRAC_PI_2)
    }

    /// Tangent over the interval; a pole inside is a domain error.
    pub fn tan(&self) -> Result<InexactInterval, ToleranceError> {
        use std::f64::consts::{FRAC_PI_2, PI};
        let (lo, hi) = self.f64_bounds()?;
        // Poles sit at pi/2 + n*pi.
        if ((hi - FRAC_PI_2) / PI).floor() > ((lo - FRAC_PI_2) / PI).floor() {
            return Err(ToleranceError::Domain("tan"));
        }
        debug!("tolerance: tan is inexact and conservative");
        Ok(outward(lo.tan(), hi.tan()))
    }

    /// `sin(x + shift)`: shift 0 is sine, pi/2 is cosine.
    fn periodic(&self, shift: f64) -> Result<InexactInterval, ToleranceError> {
        use std::f64::consts::{FRAC_PI_2, PI};
        let f = |x: f64| (x + shift).sin();
        let (lo, hi) = self.f64_bounds()?;
        let (mut mn, mut mx) = (f(lo).min(f(hi)), f(lo).max(f(hi)));
        // Interior extrema sit where x + shift = pi/2 + n*pi; two cover a period.
        let first = ((lo + shift - FRAC_PI_2) / PI).ceil();
        let last = ((hi + shift - FRAC_PI_2) / PI).floor();
        let mut n = first;
        while n <= last && n - first < 2.0 {
            let v = if (n as i64) % 2 == 0 { 1.0 } else { -1.0 };
            mn = mn.min(v);
            mx = mx.max(v);
            n += 1.0;
        }
        debug!("tolerance: trig is inexact and conservative");
        Ok(outward(mn, mx))
    }
}

/// `sqrt(a^2 + b^2)` as an upper bound, and whether it is exact.
///
/// Exact when either coefficient is zero or the sum is a rational square;
/// otherwise the smallest multiple of `1 / (den * 2^32)` not below the root,
/// so the bound never undershoots. `None` when `a`, `b` mix `pi` exponents.
fn disc_norm(a: &Magnitude, b: &Magnitude) -> Option<(Magnitude, bool)> {
    use num_bigint::BigInt;
    let abs = |m: &Magnitude| {
        if sign(m) == Ordering::Less {
            m.neg()
        } else {
            m.clone()
        }
    };
    if a.is_zero() {
        return Some((abs(b), true));
    }
    if b.is_zero() {
        return Some((abs(a), true));
    }
    let s = add(&a.mul(a), &b.mul(b)).ok()?;
    let k = s.pi_exp() / 2;
    let (n, d) = (s.rational().numer(), s.rational().denom());
    let (rn, rd) = (n.sqrt(), d.sqrt());
    if &(&rn * &rn) == n && &(&rd * &rd) == d {
        let r = Rational::new(rn, rd).ok()?;
        return Some((Magnitude::new(r, k), true));
    }
    let scale = BigInt::from(1u64 << 32);
    let m = n * d * &scale * &scale;
    let mut q = m.sqrt();
    if &q * &q < m {
        q += 1;
    }
    let r = Rational::new(q, d * &scale).ok()?;
    trace!("tolerance: disc radius is an outward rational bound");
    Some((Magnitude::new(r, k), false))
}

/// Widen an f64 interval by a few ulps so rounding never shrinks it.
fn outward(lo: f64, hi: f64) -> InexactInterval {
    let pad = |x: f64| 4.0 * f64::EPSILON * x.abs().max(f64::MIN_POSITIVE);
    InexactInterval {
        lo: lo - pad(lo),
        hi: hi + pad(hi),
        conservative: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dimension::Dimension;

    fn len() -> Dim {
        Dim::new(Dimension::from_si([1, 0, 0, 0, 0, 0, 0]), 0)
    }

    fn mass() -> Dim {
        Dim::new(Dimension::from_si([0, 1, 0, 0, 0, 0, 0]), 0)
    }

    /// `n/d` of a metre-scale magnitude (SI base).
    fn m(n: i64, d: i64) -> Magnitude {
        Magnitude::new(Rational::new(n, d).unwrap(), 0)
    }

    fn mm(n: i64, d: i64) -> Magnitude {
        m(n, d * 1000)
    }

    // frob:tests crates/cadetta-units/src/tolerance.rs::Toleranced.from_deviations kind=unit
    #[test]
    fn literals_build_ordered_deviation_pairs() {
        let sym = Toleranced::symmetric(len(), mm(10, 1), (len(), mm(5, 100))).unwrap();
        assert_eq!(sym.lower(), &mm(-5, 100));
        assert_eq!(sym.upper(), &mm(5, 100));

        let h = Toleranced::from_deviations(len(), mm(8, 1), (len(), m(0, 1)), (len(), m(0, 1)));
        assert!(h.is_ok());
        let asym =
            Toleranced::from_deviations(len(), mm(8, 1), (len(), mm(-8, 1000)), (len(), m(0, 1)))
                .unwrap();
        assert_eq!(asym.upper(), &m(0, 1));
        assert_eq!(asym.min().unwrap(), mm(7992, 1000));

        let g6 = Toleranced::from_deviations(
            len(),
            mm(12, 1),
            (len(), mm(-17, 1000)),
            (len(), mm(-6, 1000)),
        )
        .unwrap();
        assert_eq!(g6.max().unwrap(), mm(11994, 1000));
    }

    // frob:tests crates/cadetta-units/src/tolerance.rs::ToleranceError kind=unit
    #[test]
    fn inverted_or_mismatched_deviations_are_errors() {
        let inv =
            Toleranced::from_deviations(len(), mm(10, 1), (len(), mm(1, 10)), (len(), mm(-1, 10)));
        assert_eq!(inv, Err(ToleranceError::InvertedDeviations));
        let bad = Toleranced::symmetric(len(), mm(10, 1), (mass(), m(1, 10)));
        assert!(matches!(bad, Err(ToleranceError::DimensionMismatch { .. })));
        let bad_nominal = Toleranced::symmetric(len(), mm(10, 1), (len(), m(1, 10)))
            .and_then(|_| Toleranced::from_interval(len(), mm(2, 1), mm(1, 1)));
        assert_eq!(bad_nominal, Err(ToleranceError::InvertedDeviations));
    }

    // frob:tests crates/cadetta-units/src/tolerance.rs::Toleranced.symmetric kind=unit
    #[test]
    fn deviation_units_may_differ_from_nominal() {
        // 1in = 127/5000 m nominal, 0.1mm deviation.
        let t = Toleranced::symmetric(len(), m(127, 5000), (len(), mm(1, 10))).unwrap();
        assert_eq!(t.max().unwrap(), add(&m(127, 5000), &mm(1, 10)).unwrap());
    }

    // frob:tests crates/cadetta-units/src/tolerance.rs::Toleranced.from_interval kind=unit
    #[test]
    fn interval_nominal_is_midpoint() {
        let t = Toleranced::from_interval(len(), mm(998, 100), mm(1005, 100)).unwrap();
        assert_eq!(t.nominal(), &mm(10015, 1000));
        assert_eq!(t.mid().unwrap(), *t.nominal());
        assert_eq!(t.min().unwrap(), mm(998, 100));
        assert_eq!(t.max().unwrap(), mm(1005, 100));
    }

    fn var(n: u32, tol_mm: i64) -> Affine {
        Toleranced::symmetric(len(), mm(10, 1), (len(), mm(tol_mm, 100)))
            .unwrap()
            .to_affine(VarId(n))
            .unwrap()
    }

    fn disc(n: u32, a: Magnitude, b: Magnitude) -> Affine {
        Affine::disc(len(), m(0, 1), VarId(n), a, b)
    }

    // frob:tests crates/cadetta-units/src/tolerance.rs::Affine.disc kind=unit
    // frob:tests crates/cadetta-units/src/tolerance.rs::Affine.bounds kind=unit
    #[test]
    fn disc_bounds_use_euclidean_radius() {
        // 3-4-5: exact and not conservative, tighter than the L1 box (7).
        let f = disc(0, mm(3, 1), mm(4, 1));
        assert_eq!(f.bounds().unwrap(), (mm(-5, 1), mm(5, 1)));
        assert!(!f.is_conservative() && !f.is_exact());
        assert_eq!(f.disc_coefficients(VarId(0)), Some((&mm(3, 1), &mm(4, 1))));
        // one zero coefficient: exact |a|
        let g = disc(0, mm(-3, 1), m(0, 1));
        assert_eq!(g.bounds().unwrap(), (mm(-3, 1), mm(3, 1)));
        assert!(!g.is_conservative());
        // irrational radius: outward bound, conservative
        let h = disc(0, mm(1, 1), mm(1, 1));
        let (lo, hi) = h.bounds().unwrap();
        assert!(h.is_conservative());
        assert!(
            hi.to_f64() >= 2f64.sqrt() / 1000.0 && hi.to_f64() < 1.0000001 * 2f64.sqrt() / 1000.0
        );
        assert_eq!(lo, hi.neg());
        // all-zero pair carries no term
        assert!(disc(0, m(0, 1), m(0, 1)).is_exact());
    }

    // frob:tests crates/cadetta-units/src/tolerance.rs::Affine.add kind=unit
    // frob:tests crates/cadetta-units/src/tolerance.rs::Affine.neg kind=unit
    #[test]
    fn disc_terms_combine_cancel_and_scale() {
        let f = disc(0, mm(3, 1), mm(4, 1));
        assert!(f.sub(&f).unwrap().is_exact());
        let twice = f.add(&f).unwrap();
        assert_eq!(
            twice.disc_coefficients(VarId(0)),
            Some((&mm(6, 1), &mm(8, 1)))
        );
        assert_eq!(twice.bounds().unwrap(), (mm(-10, 1), mm(10, 1)));
        assert_eq!(
            f.neg().disc_coefficients(VarId(0)),
            Some((&mm(-3, 1), &mm(-4, 1)))
        );
        // scale by an exact constant, and the box term adds on top
        let k = Affine::constant(Dim::default(), m(2, 1));
        let s = f.mul(&k).unwrap();
        assert_eq!(s.disc_coefficients(VarId(0)), Some((&mm(6, 1), &mm(8, 1))));
        let boxed = Toleranced::symmetric(len(), m(0, 1), (len(), mm(1, 1)))
            .unwrap()
            .to_affine(VarId(1))
            .unwrap();
        let both = f.add(&boxed).unwrap();
        assert_eq!(both.bounds().unwrap(), (mm(-6, 1), mm(6, 1)));
    }

    // frob:tests crates/cadetta-units/src/tolerance.rs::Affine.mul kind=unit
    // frob:tests crates/cadetta-units/src/tolerance.rs::Affine.div kind=unit
    #[test]
    fn disc_nonlinear_falls_back_to_hull() {
        let f = disc(0, mm(3, 1), mm(4, 1));
        let p = f.mul(&f).unwrap();
        assert!(p.is_conservative() && p.disc_coefficients(VarId(0)).is_none());
        let shifted = Affine::disc(len(), m(1, 1), VarId(0), mm(3, 1), mm(4, 1));
        let q = shifted.div(&shifted).unwrap();
        assert!(q.is_conservative());
    }

    // frob:tests crates/cadetta-units/src/tolerance.rs::Affine.sub kind=unit
    #[test]
    fn repeated_terms_count_once_and_cancel() {
        let a = var(0, 5);
        let twice = a.add(&a).unwrap();
        assert_eq!(twice.coefficient(VarId(0)), Some(&mm(10, 100)));
        assert!(!twice.is_conservative());
        let zero = a.sub(&a).unwrap();
        assert!(zero.is_exact());
        assert_eq!(zero.bounds().unwrap(), (m(0, 1), m(0, 1)));
        // Independent variables add worst-case.
        let b = var(1, 5);
        let (lo, hi) = a.add(&b).unwrap().bounds().unwrap();
        assert_eq!((lo, hi), (mm(1990, 100), mm(2010, 100)));
    }

    // frob:tests crates/cadetta-units/src/tolerance.rs::Affine.mul kind=unit
    #[test]
    fn product_of_toleranced_values_is_conservative() {
        let a = var(0, 100); // 10mm +-1mm
        let p = a.mul(&var(1, 100)).unwrap();
        assert!(p.is_conservative());
        let (lo, hi) = p.bounds().unwrap();
        assert_eq!(lo, m(81, 1_000_000));
        assert_eq!(hi, m(121, 1_000_000));
        // Scaling by an exact constant stays linear and exact.
        let k = Affine::constant(Dim::default(), m(2, 1));
        let s = a.mul(&k).unwrap();
        assert!(!s.is_conservative());
        assert_eq!(s.coefficient(VarId(0)), Some(&mm(2, 1)));
        let q = a.div(&var(1, 100)).unwrap();
        assert!(q.is_conservative());
    }

    // frob:tests crates/cadetta-units/src/tolerance.rs::Affine.cbrt kind=unit
    #[test]
    fn cbrt_interval_is_conservative_and_handles_negatives() {
        let a = Toleranced::symmetric(Dim::default(), m(-1, 1), (Dim::default(), m(7, 1)))
            .unwrap()
            .to_affine(VarId(0))
            .unwrap();
        let r = a.cbrt().unwrap();
        assert!(r.conservative);
        assert!(r.lo <= (-8f64).cbrt() && r.hi >= 6f64.cbrt());
    }

    // frob:tests crates/cadetta-units/src/tolerance.rs::Affine.sqrt kind=unit
    #[test]
    fn roots_and_trig_are_conservative() {
        let a = Toleranced::symmetric(Dim::default(), m(4, 1), (Dim::default(), m(1, 1)))
            .unwrap()
            .to_affine(VarId(0))
            .unwrap();
        let r = a.sqrt().unwrap();
        assert!(r.conservative);
        assert!(r.lo <= 3f64.sqrt() && r.hi >= 5f64.sqrt());
        let neg = Affine::constant(Dim::default(), m(-1, 1)).sqrt();
        assert_eq!(neg, Err(ToleranceError::Domain("sqrt")));

        let t = Toleranced::symmetric(Dim::default(), m(1, 1), (Dim::default(), m(3, 5)))
            .unwrap()
            .to_affine(VarId(0))
            .unwrap();
        let s = t.sin().unwrap();
        assert!(s.conservative && s.lo <= 0.4f64.sin() && s.hi >= 1.6f64.sin());
        // Interval 0.4..1.6 contains pi/2, so the sine peaks at 1.
        assert!(s.hi >= 1.0);
        let c = t.cos().unwrap();
        assert!(c.conservative && c.hi >= 0.4f64.cos());
        assert!(t.tan().is_err());
    }
}
