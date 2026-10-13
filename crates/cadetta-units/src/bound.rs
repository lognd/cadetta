// frob:ticket 01M4ST7A42Q225NZMARTM5V6J5
//! Bounds: one- or two-sided requirements, never values (quantities.md 5.7).
//!
//! `x <= 8.5mm`, `x >= 9.6mm` and `x in 4mm..6mm` build a [`Bound`] whose
//! `min`/`max` are its limits. A bound is never emitted to a CAD tool.

use std::cmp::Ordering;

use thiserror::Error;
use tracing::{debug, trace};

use crate::dimension::Dim;
use crate::rational::Magnitude;
use crate::tolerance::cmp_mag;

/// Errors from building a bound.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum BoundError {
    /// A range `lo..hi` with `lo > hi`.
    #[error("range lower limit exceeds upper limit")]
    InvertedRange,
}

/// A requirement on a quantity: an optional lower and upper limit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bound {
    dim: Dim,
    min: Option<Magnitude>,
    max: Option<Magnitude>,
}

impl Bound {
    /// `x <= limit`: only an upper limit.
    pub fn at_most(dim: Dim, limit: Magnitude) -> Self {
        trace!("bound: at_most");
        Self {
            dim,
            min: None,
            max: Some(limit),
        }
    }

    /// `x >= limit`: only a lower limit.
    pub fn at_least(dim: Dim, limit: Magnitude) -> Self {
        trace!("bound: at_least");
        Self {
            dim,
            min: Some(limit),
            max: None,
        }
    }

    /// `x in lo..hi`: both limits.
    pub fn within(dim: Dim, lo: Magnitude, hi: Magnitude) -> Result<Self, BoundError> {
        if cmp_mag(&lo, &hi) == Ordering::Greater {
            debug!("bound: inverted range");
            return Err(BoundError::InvertedRange);
        }
        Ok(Self {
            dim,
            min: Some(lo),
            max: Some(hi),
        })
    }

    /// The dimension of the limits.
    pub fn dim(&self) -> &Dim {
        &self.dim
    }

    /// `P.min`: the lower limit, absent for a one-sided `<=` bound.
    pub fn min(&self) -> Option<&Magnitude> {
        self.min.as_ref()
    }

    /// `P.max`: the upper limit, absent for a one-sided `>=` bound.
    pub fn max(&self) -> Option<&Magnitude> {
        self.max.as_ref()
    }

    /// Always false: a bound is a requirement and is never emitted (5.7).
    pub fn is_emittable(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dimension::Dimension;
    use crate::rational::Rational;

    fn len() -> Dim {
        Dim::new(Dimension::from_si([1, 0, 0, 0, 0, 0, 0]), 0)
    }

    fn mm(n: i64, d: i64) -> Magnitude {
        Magnitude::new(Rational::new(n, d).unwrap(), 0)
    }

    // frob:tests crates/cadetta-units/src/bound.rs::Bound.at_most kind=unit
    #[test]
    fn at_most_has_only_max() {
        let b = Bound::at_most(len(), mm(85, 10_000));
        assert_eq!(b.min(), None);
        assert_eq!(b.max(), Some(&mm(85, 10_000)));
        assert!(!b.is_emittable());
    }

    // frob:tests crates/cadetta-units/src/bound.rs::Bound.at_least kind=unit
    #[test]
    fn at_least_has_only_min() {
        let b = Bound::at_least(len(), mm(96, 10_000));
        assert_eq!(b.min(), Some(&mm(96, 10_000)));
        assert_eq!(b.max(), None);
        assert!(!b.is_emittable());
    }

    // frob:tests crates/cadetta-units/src/bound.rs::Bound.within kind=unit
    #[test]
    fn range_has_both_limits_and_rejects_inversion() {
        let b = Bound::within(len(), mm(4, 1000), mm(6, 1000)).unwrap();
        assert_eq!(b.min(), Some(&mm(4, 1000)));
        assert_eq!(b.max(), Some(&mm(6, 1000)));
        assert!(!b.is_emittable());
        assert_eq!(
            Bound::within(len(), mm(6, 1000), mm(4, 1000)),
            Err(BoundError::InvertedRange)
        );
    }
}
