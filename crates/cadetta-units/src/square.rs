//! Square-aware interval evaluation for `x ^ 2` and `x * x`.
//!
//! Specification: `docs/spec/quantities.md` 5.4. The generic product
//! treats its factors as independent, so `x * x` over an `x` that
//! straddles zero would give a negative lower bound. Squaring the same
//! form is never negative, which keeps `sqrt(x^2 + y^2)` defined.

use std::cmp::Ordering;

use tracing::debug;

use crate::rational::Magnitude;
use crate::tolerance::{Affine, ToleranceError, cmp_mag, sign};

/// `x ^ 2` as a square-aware interval: never negative, conservative
/// unless `x` is exact (then it is the exact square).
pub fn square(x: &Affine) -> Result<Affine, ToleranceError> {
    if x.is_exact() {
        return x.mul(x);
    }
    let dim = x.dim().mul(x.dim());
    let (a, b) = x.bounds()?;
    let (a2, b2) = (a.mul(&a), b.mul(&b));
    let zero = Magnitude::new(crate::rational::Rational::from_int(0), 0);
    let (lo, hi) = match (sign(&a), sign(&b)) {
        (Ordering::Less, Ordering::Greater) => {
            let hi = if cmp_mag(&a2, &b2) == Ordering::Less {
                b2
            } else {
                a2
            };
            debug!("square: interval straddles zero, lower bound clamped to 0");
            (zero, hi)
        }
        (Ordering::Less, _) => (b2, a2),
        _ => (a2, b2),
    };
    Affine::from_hull(dim, lo, hi)
}

/// `x * y`, using the square-aware interval when both operands are the
/// same affine form and the ordinary product otherwise.
pub fn mul_checked(x: &Affine, y: &Affine) -> Result<Affine, ToleranceError> {
    if x == y {
        debug!("square: identical operands use the square-aware interval");
        square(x)
    } else {
        x.mul(y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dimension::{Dim, Dimension};
    use crate::rational::Rational;
    use crate::tolerance::{Toleranced, VarId};

    fn len() -> Dim {
        Dim::new(Dimension::from_si([1, 0, 0, 0, 0, 0, 0]), 0)
    }

    fn m(n: i64, d: i64) -> Magnitude {
        Magnitude::new(Rational::new(n, d).unwrap(), 0)
    }

    /// `nominal +- tol` metres on source variable `v`.
    fn var(v: u32, nominal: i64, tol: i64) -> Affine {
        Toleranced::symmetric(len(), m(nominal, 1), (len(), m(tol, 1)))
            .unwrap()
            .to_affine(VarId(v))
            .unwrap()
    }

    // frob:tests crates/cadetta-units/src/square.rs::square kind=unit
    #[test]
    fn square_is_never_negative() {
        let x = var(0, 1, 3); // [-2, 4]
        let naive = x.mul(&x).unwrap();
        assert_eq!(
            cmp_mag(&naive.bounds().unwrap().0, &m(0, 1)),
            Ordering::Less
        );
        let sq = mul_checked(&x, &x).unwrap();
        assert!(sq.is_conservative());
        assert_eq!(sq.bounds().unwrap(), (m(0, 1), m(16, 1)));
        // One-sided intervals keep their tight bounds.
        assert_eq!(
            square(&var(0, 5, 2)).unwrap().bounds().unwrap(),
            (m(9, 1), m(49, 1))
        );
        assert_eq!(
            square(&var(0, -5, 2)).unwrap().bounds().unwrap(),
            (m(9, 1), m(49, 1))
        );
        // An exact operand squares exactly.
        let e = Affine::constant(len(), m(3, 1));
        let es = square(&e).unwrap();
        assert!(es.is_exact() && !es.is_conservative());
        assert_eq!(es.center(), &m(9, 1));
    }

    // frob:tests crates/cadetta-units/src/square.rs::square kind=unit
    #[test]
    fn sqrt_of_sum_of_squares_stays_defined() {
        let x = var(0, 0, 2); // straddles zero
        let y = var(1, 3, 1);
        let sum = square(&x).unwrap().add(&square(&y).unwrap()).unwrap();
        assert!(sum.is_conservative());
        let r = sum.sqrt().unwrap();
        assert!(r.conservative);
        assert!(r.lo <= 2.0 && r.hi >= 20f64.sqrt());
        // The naive product is undefined here.
        let y0 = var(2, 1, 1); // [0, 2]
        let naive = x.mul(&x).unwrap().add(&y0.mul(&y0).unwrap()).unwrap();
        assert_eq!(naive.sqrt(), Err(ToleranceError::Domain("sqrt")));
    }

    // frob:tests crates/cadetta-units/src/square.rs::mul_checked kind=unit
    #[test]
    fn distinct_forms_use_the_ordinary_product() {
        let x = var(0, 1, 3);
        let y = var(1, 1, 3); // same numbers, different source variable
        let p = mul_checked(&x, &y).unwrap();
        assert!(p.is_conservative());
        assert_eq!(p.bounds().unwrap(), x.mul(&y).unwrap().bounds().unwrap());
        assert_eq!(p.bounds().unwrap().0, m(-8, 1));
    }
}
