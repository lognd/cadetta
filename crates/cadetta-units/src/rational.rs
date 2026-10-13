// frob:ticket 01M4SPWHVJVHRD0KWP8ESJD81M
//! Exact magnitudes `r * pi^k` (quantities.md 1): arbitrary-precision
//! rationals with an integer power of pi, so `360deg / 1.8deg` is exactly 200.

use std::fmt;

use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{One, Signed, ToPrimitive, Zero};
use thiserror::Error;
use tracing::{debug, trace};

/// Failure of an exact magnitude operation.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum MagnitudeError {
    /// A rational was built with a zero denominator.
    #[error("zero denominator")]
    ZeroDenominator,
    /// Division by zero (or a negative power of zero).
    #[error("division by zero")]
    DivisionByZero,
    /// A decimal literal could not be parsed.
    #[error("invalid decimal literal `{0}`")]
    InvalidDecimal(String),
}

/// Arbitrary-precision rational in lowest terms with a positive denominator.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Rational {
    num: BigInt,
    den: BigInt,
}

impl Rational {
    /// Build `num/den` in lowest terms; a zero denominator is an error.
    pub fn new(num: impl Into<BigInt>, den: impl Into<BigInt>) -> Result<Self, MagnitudeError> {
        let (mut num, mut den) = (num.into(), den.into());
        if den.is_zero() {
            debug!("rational: rejected zero denominator");
            return Err(MagnitudeError::ZeroDenominator);
        }
        if den.is_negative() {
            num = -num;
            den = -den;
        }
        let g = num.gcd(&den);
        if !g.is_one() && !g.is_zero() {
            num /= &g;
            den /= &g;
        }
        if num.is_zero() {
            den = BigInt::one();
        }
        Ok(Self { num, den })
    }

    /// The integer `n` as a rational.
    pub fn from_int(n: impl Into<BigInt>) -> Self {
        Self {
            num: n.into(),
            den: BigInt::one(),
        }
    }

    /// Parse an exact decimal such as `-12.5` or `1e-3` without any float.
    pub fn parse_decimal(text: &str) -> Result<Self, MagnitudeError> {
        let bad = || MagnitudeError::InvalidDecimal(text.to_string());
        let lower = text.to_ascii_lowercase();
        let (mantissa, exp) = match lower.split_once('e') {
            Some((m, e)) => (m, e.parse::<i32>().map_err(|_| bad())?),
            None => (lower.as_str(), 0),
        };
        let (int_part, frac_part) = mantissa.split_once('.').unwrap_or((mantissa, ""));
        let body = int_part.trim_start_matches(['+', '-']);
        let digits_ok = |s: &str| s.bytes().all(|b| b.is_ascii_digit());
        let dot = mantissa.contains('.');
        // A leading or trailing `.` is a lex error (quantities.md 2).
        if body.is_empty()
            || (dot && frac_part.is_empty())
            || !digits_ok(body)
            || !digits_ok(frac_part)
        {
            return Err(bad());
        }
        let negative = int_part.starts_with('-');
        let all = format!("{body}{frac_part}");
        let mut n: BigInt = all.parse().map_err(|_| bad())?;
        if negative {
            n = -n;
        }
        let scale = exp - i32::try_from(frac_part.len()).map_err(|_| bad())?;
        let pow = BigInt::from(10).pow(scale.unsigned_abs());
        if scale >= 0 {
            Ok(Self::from_int(n * pow))
        } else {
            Self::new(n, pow)
        }
    }

    /// Numerator (sign carrier).
    pub fn numer(&self) -> &BigInt {
        &self.num
    }

    /// Denominator (always positive).
    pub fn denom(&self) -> &BigInt {
        &self.den
    }

    /// True when the value is zero.
    pub fn is_zero(&self) -> bool {
        self.num.is_zero()
    }

    /// Exact sum.
    pub fn add(&self, o: &Self) -> Self {
        Self::reduced(&self.num * &o.den + &o.num * &self.den, &self.den * &o.den)
    }

    /// Exact difference.
    pub fn sub(&self, o: &Self) -> Self {
        Self::reduced(&self.num * &o.den - &o.num * &self.den, &self.den * &o.den)
    }

    /// Exact product.
    pub fn mul(&self, o: &Self) -> Self {
        Self::reduced(&self.num * &o.num, &self.den * &o.den)
    }

    /// Exact quotient; dividing by zero is an error.
    pub fn div(&self, o: &Self) -> Result<Self, MagnitudeError> {
        if o.is_zero() {
            return Err(MagnitudeError::DivisionByZero);
        }
        Self::new(&self.num * &o.den, &self.den * &o.num)
    }

    /// Exact negation.
    pub fn neg(&self) -> Self {
        Self {
            num: -&self.num,
            den: self.den.clone(),
        }
    }

    /// Exact integer power; a negative power of zero is an error.
    pub fn powi(&self, n: i32) -> Result<Self, MagnitudeError> {
        let p = n.unsigned_abs();
        let r = Self::reduced(self.num.pow(p), self.den.pow(p));
        if n >= 0 { Ok(r) } else { Self::one().div(&r) }
    }

    /// The rational 1.
    pub fn one() -> Self {
        Self::from_int(1)
    }

    /// Nearest binary64 (inexact; for marked-inexact results only).
    pub fn to_f64(&self) -> f64 {
        match (self.num.to_f64(), self.den.to_f64()) {
            (Some(n), Some(d)) if n.is_finite() && d.is_finite() => n / d,
            _ => {
                // Huge parts: shift both down to keep the ratio finite.
                let shift = self.num.bits().max(self.den.bits()).saturating_sub(1000);
                let n = (&self.num >> shift).to_f64().unwrap_or(f64::NAN);
                let d = (&self.den >> shift).to_f64().unwrap_or(f64::NAN);
                n / d
            }
        }
    }

    /// Build from parts known to have a nonzero denominator.
    fn reduced(num: BigInt, den: BigInt) -> Self {
        Self::new(num, den).expect("denominator product of nonzero denominators")
    }
}

impl fmt::Display for Rational {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.den.is_one() {
            write!(f, "{}", self.num)
        } else {
            write!(f, "{}/{}", self.num, self.den)
        }
    }
}

impl fmt::Debug for Rational {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Rational({self})")
    }
}

/// Result of adding magnitudes: exact, or marked inexact as a binary64.
#[derive(Debug, Clone, PartialEq)]
pub enum Sum {
    /// Same pi exponent (or one operand zero): the sum is exact.
    Exact(Magnitude),
    /// Different nonzero pi exponents: carried as an IEEE-754 binary64.
    Inexact(f64),
}

/// An exact magnitude `r * pi^k` in SI base units (quantities.md 1).
///
/// Zero is normalised to `k = 0`, so equality is structural.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Magnitude {
    r: Rational,
    k: i32,
}

impl Magnitude {
    /// Build `r * pi^k`, normalising zero to exponent 0.
    pub fn new(r: Rational, k: i32) -> Self {
        let k = if r.is_zero() { 0 } else { k };
        Self { r, k }
    }

    /// The rational factor `r`.
    pub fn rational(&self) -> &Rational {
        &self.r
    }

    /// The integer exponent of pi.
    pub fn pi_exp(&self) -> i32 {
        self.k
    }

    /// True when the magnitude is zero.
    pub fn is_zero(&self) -> bool {
        self.r.is_zero()
    }

    /// Add; exact when the exponents match or one side is zero, else inexact.
    pub fn add(&self, o: &Self) -> Sum {
        if self.is_zero() {
            return Sum::Exact(o.clone());
        }
        if o.is_zero() {
            return Sum::Exact(self.clone());
        }
        if self.k == o.k {
            return Sum::Exact(Self::new(self.r.add(&o.r), self.k));
        }
        trace!(
            k1 = self.k,
            k2 = o.k,
            "magnitude: add across pi exponents is inexact"
        );
        Sum::Inexact(self.to_f64() + o.to_f64())
    }

    /// Subtract; same exactness rule as [`Magnitude::add`].
    pub fn sub(&self, o: &Self) -> Sum {
        self.add(&o.neg())
    }

    /// Exact negation.
    pub fn neg(&self) -> Self {
        Self::new(self.r.neg(), self.k)
    }

    /// Exact product: rationals multiply, exponents add.
    pub fn mul(&self, o: &Self) -> Self {
        Self::new(self.r.mul(&o.r), self.k + o.k)
    }

    /// Exact quotient: exponents subtract, so `pi^k` cancels when equal.
    pub fn div(&self, o: &Self) -> Result<Self, MagnitudeError> {
        Ok(Self::new(self.r.div(&o.r)?, self.k - o.k))
    }

    /// Exact integer power.
    pub fn powi(&self, n: i32) -> Result<Self, MagnitudeError> {
        Ok(Self::new(self.r.powi(n)?, self.k * n))
    }

    /// Nearest binary64 value (inexact; never stored as a magnitude).
    pub fn to_f64(&self) -> f64 {
        self.r.to_f64() * std::f64::consts::PI.powi(self.k)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q(n: i64, d: i64) -> Rational {
        Rational::new(n, d).unwrap()
    }
    fn dec(s: &str) -> Rational {
        Rational::parse_decimal(s).unwrap()
    }

    // frob:tests crates/cadetta-units/src/rational.rs::Magnitude.new kind=unit
    #[test]
    fn one_inch_is_exactly_127_over_5000_metres() {
        let inch = Magnitude::new(dec("25.4").mul(&dec("1e-3")), 0);
        assert_eq!(inch.rational(), &q(127, 5000));
        assert_eq!(inch.pi_exp(), 0);
    }

    // frob:tests crates/cadetta-units/src/rational.rs::Magnitude.div kind=unit
    #[test]
    fn degrees_ratio_cancels_pi_exactly() {
        let deg = Magnitude::new(q(1, 180), 1);
        let a = deg.mul(&Magnitude::new(dec("360"), 0));
        let b = deg.mul(&Magnitude::new(dec("1.8"), 0));
        let ratio = a.div(&b).unwrap();
        assert_eq!(ratio, Magnitude::new(Rational::from_int(200), 0));
    }

    // frob:tests crates/cadetta-units/src/rational.rs::Magnitude.add kind=unit
    #[test]
    fn add_across_pi_exponents_is_inexact_unless_zero() {
        let a = Magnitude::new(q(1, 2), 0);
        let b = Magnitude::new(q(1, 3), 1);
        assert!(matches!(a.add(&b), Sum::Inexact(_)));
        let zero = Magnitude::new(Rational::from_int(0), 5);
        assert_eq!(a.add(&zero), Sum::Exact(a.clone()));
        assert_eq!(zero.add(&b), Sum::Exact(b.clone()));
        let c = Magnitude::new(q(1, 6), 1);
        assert_eq!(b.add(&c), Sum::Exact(Magnitude::new(q(1, 2), 1)));
    }

    #[test]
    fn division_by_zero_is_an_error_value() {
        let z = Magnitude::new(Rational::from_int(0), 0);
        assert_eq!(z.div(&z), Err(MagnitudeError::DivisionByZero));
        assert_eq!(Rational::new(1, 0), Err(MagnitudeError::ZeroDenominator));
    }

    #[test]
    fn powers_and_decimals_are_exact() {
        let m = Magnitude::new(q(2, 3), 1);
        assert_eq!(m.powi(-2).unwrap(), Magnitude::new(q(9, 4), -2));
        assert_eq!(dec("-12.5"), q(-25, 2));
        assert_eq!(dec("1e-3"), q(1, 1000));
        assert!(Rational::parse_decimal("1.").is_err());
        assert!(Rational::parse_decimal("abc").is_err());
    }
}
