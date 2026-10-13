// frob:ticket 01M4SPWHVJVHRD0KWP8ESJD81M
//! Dimension vectors (quantities.md 1, 3): seven SI base exponents, one base
//! dimension per currency, and an integer angle exponent tracked beside them.

use std::collections::BTreeMap;

use thiserror::Error;
use tracing::debug;

/// Number of SI base dimensions `[L, M, T, I, Theta, N, J]`.
pub const SI_BASE_COUNT: usize = 7;

/// Dimension mismatch, returned as a value (never a panic).
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DimensionError {
    /// Operands of `+ -` or a comparison differ in dimension.
    #[error("dimension mismatch: {left} vs {right}")]
    Mismatch {
        /// Rendering of the left operand's dimension.
        left: String,
        /// Rendering of the right operand's dimension.
        right: String,
    },
}

/// Exponent vector over the SI bases plus one base per currency code.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct Dimension {
    si: [i32; SI_BASE_COUNT],
    currency: BTreeMap<String, i32>,
}

impl Dimension {
    /// The dimensionless vector.
    pub fn dimensionless() -> Self {
        Self::default()
    }

    /// A vector from explicit SI exponents `[L, M, T, I, Theta, N, J]`.
    pub fn from_si(si: [i32; SI_BASE_COUNT]) -> Self {
        Self {
            si,
            currency: BTreeMap::new(),
        }
    }

    /// A currency (ISO 4217 code) as its own base dimension, exponent 1.
    pub fn currency(code: &str) -> Self {
        let mut currency = BTreeMap::new();
        currency.insert(code.to_string(), 1);
        Self {
            si: [0; SI_BASE_COUNT],
            currency,
        }
    }

    /// The SI exponents.
    pub fn si(&self) -> &[i32; SI_BASE_COUNT] {
        &self.si
    }

    /// The exponent of one currency (0 when absent).
    pub fn currency_exp(&self, code: &str) -> i32 {
        self.currency.get(code).copied().unwrap_or(0)
    }

    /// True when every exponent is zero.
    pub fn is_dimensionless(&self) -> bool {
        self.si.iter().all(|e| *e == 0) && self.currency.is_empty()
    }

    /// Combine exponents with `f`, dropping zero currency entries.
    fn zip(&self, o: &Self, f: impl Fn(i32, i32) -> i32) -> Self {
        let mut si = [0; SI_BASE_COUNT];
        for (i, slot) in si.iter_mut().enumerate() {
            *slot = f(self.si[i], o.si[i]);
        }
        let mut currency = BTreeMap::new();
        for code in self.currency.keys().chain(o.currency.keys()) {
            let e = f(self.currency_exp(code), o.currency_exp(code));
            if e != 0 {
                currency.insert(code.clone(), e);
            }
        }
        Self { si, currency }
    }

    /// Product dimension (exponents add).
    pub fn mul(&self, o: &Self) -> Self {
        self.zip(o, |a, b| a + b)
    }

    /// Quotient dimension (exponents subtract).
    pub fn div(&self, o: &Self) -> Self {
        self.zip(o, |a, b| a - b)
    }

    /// Integer power (exponents multiply).
    pub fn powi(&self, n: i32) -> Self {
        let none = Self::default();
        let mut r = none.zip(self, |_, b| b * n);
        r.currency.retain(|_, e| *e != 0);
        r
    }

    // frob:ticket 01M4SPWNHPTRETQBYCM6EA2GEV
    /// Exact `n`-th root when every exponent divides by `n`, else `None` (`n` > 0).
    pub fn root(&self, n: i32) -> Option<Self> {
        if n <= 0
            || self
                .si
                .iter()
                .chain(self.currency.values())
                .any(|e| e % n != 0)
        {
            return None;
        }
        let mut si = self.si;
        si.iter_mut().for_each(|e| *e /= n);
        let currency = self
            .currency
            .iter()
            .map(|(c, e)| (c.clone(), e / n))
            .collect();
        Some(Self { si, currency })
    }

    /// Require equal dimensions, as `+ -` and comparisons do.
    pub fn check_same(&self, o: &Self) -> Result<(), DimensionError> {
        if self == o {
            return Ok(());
        }
        debug!(left = %self.render(), right = %o.render(), "dimension mismatch");
        Err(DimensionError::Mismatch {
            left: self.render(),
            right: o.render(),
        })
    }

    /// Human-readable form such as `L^1 M^1 T^-2 USD^1`; `1` when dimensionless.
    pub fn render(&self) -> String {
        const NAMES: [&str; SI_BASE_COUNT] = ["L", "M", "T", "I", "Theta", "N", "J"];
        let mut parts: Vec<String> = Vec::new();
        for (n, e) in NAMES.iter().zip(self.si) {
            if e != 0 {
                parts.push(format!("{n}^{e}"));
            }
        }
        for (c, e) in &self.currency {
            parts.push(format!("{c}^{e}"));
        }
        if parts.is_empty() {
            "1".to_string()
        } else {
            parts.join(" ")
        }
    }
}

/// A dimension together with its integer angle exponent (quantities.md 4.3).
///
/// The angle exponent never touches the magnitude (`1rad` is 1).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct Dim {
    /// SI and currency exponents.
    pub dimension: Dimension,
    /// Integer angle exponent.
    pub angle: i32,
}

impl Dim {
    /// Pair a dimension with an angle exponent.
    pub fn new(dimension: Dimension, angle: i32) -> Self {
        Self { dimension, angle }
    }

    /// Product: dimension and angle exponents add.
    pub fn mul(&self, o: &Self) -> Self {
        Self::new(self.dimension.mul(&o.dimension), self.angle + o.angle)
    }

    /// Quotient: dimension and angle exponents subtract.
    pub fn div(&self, o: &Self) -> Self {
        Self::new(self.dimension.div(&o.dimension), self.angle - o.angle)
    }

    /// Integer power of both parts.
    pub fn powi(&self, n: i32) -> Self {
        Self::new(self.dimension.powi(n), self.angle * n)
    }

    /// Require equal dimensions; the angle exponent is checked separately (Q008).
    pub fn check_same_dimension(&self, o: &Self) -> Result<(), DimensionError> {
        self.dimension.check_same(&o.dimension)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rational::{Magnitude, Rational};

    const LENGTH: [i32; 7] = [1, 0, 0, 0, 0, 0, 0];
    const MASS: [i32; 7] = [0, 1, 0, 0, 0, 0, 0];

    // frob:tests crates/cadetta-units/src/dimension.rs::Dimension.check_same kind=unit
    #[test]
    fn length_plus_mass_is_an_error_value() {
        let r = Dimension::from_si(LENGTH).check_same(&Dimension::from_si(MASS));
        assert!(matches!(r, Err(DimensionError::Mismatch { .. })));
        assert!(
            Dimension::from_si(LENGTH)
                .check_same(&Dimension::from_si(LENGTH))
                .is_ok()
        );
    }

    // frob:tests crates/cadetta-units/src/dimension.rs::Dimension.currency kind=unit
    #[test]
    fn currency_is_its_own_base_dimension() {
        let usd = Dimension::currency("USD");
        assert_eq!(usd.si(), &[0; SI_BASE_COUNT]);
        assert_eq!(usd.currency_exp("USD"), 1);
        assert!(!usd.is_dimensionless());
        assert!(usd.check_same(&Dimension::currency("EUR")).is_err());
        assert!(usd.div(&usd).is_dimensionless());
        assert_eq!(usd.powi(2).render(), "USD^2");
    }

    // frob:tests crates/cadetta-units/src/dimension.rs::Dim kind=unit
    #[test]
    fn angle_exponent_is_beside_dimension_and_leaves_magnitude() {
        let one = Magnitude::new(Rational::from_int(1), 0);
        let rad = Dim::new(Dimension::dimensionless(), 1);
        assert_eq!(rad.angle, 1);
        assert!(rad.dimension.is_dimensionless());
        assert_eq!(one.mul(&one), one);
        let lead = Dim::new(Dimension::from_si(LENGTH), 0).div(&rad);
        assert_eq!(lead.angle, -1);
        assert_eq!(lead.dimension, Dimension::from_si(LENGTH));
        assert!(
            lead.check_same_dimension(&Dim::new(Dimension::from_si(LENGTH), 0))
                .is_ok()
        );
    }
}
