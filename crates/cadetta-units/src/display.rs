// frob:ticket 01M4SPWFZP4041AN6YQE08X2AN
//! Display unit selection and fractional formats (quantities.md 6).
//!
//! Storage is exact SI; this module only chooses and renders a presentation.
//! Everything is exact rational arithmetic: rounding is half away from zero on
//! integers, never through an `f64`.

use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{One, Signed, Zero};
use thiserror::Error;
use tracing::{debug, trace};

use crate::dimension::Dim;
use crate::rational::{Magnitude, MagnitudeError, Rational};
use crate::units::{UnitError, UnitTable};

/// Default denominator of `ft_in` when none is written (`ft_in` means `ft_in/16`).
pub const FT_IN_DEFAULT_DENOM: u32 = 16;

/// Decimal places printed for a unit with no fixed precision when the value is not exact.
const DEFAULT_DECIMALS: u32 = 6;

/// Errors from parsing or applying a display spec, returned as values.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DisplayError {
    /// The spec text is empty or malformed.
    #[error("invalid display spec `{0}`")]
    Invalid(String),
    /// A fraction denominator is zero.
    #[error("fraction denominator must be at least 1 in `{0}`")]
    ZeroDenominator(String),
    /// The unit does not resolve.
    #[error(transparent)]
    Unit(#[from] UnitError),
    /// The display unit's dimension differs from the value's.
    #[error("display unit `{unit}` does not match the value's dimension")]
    DimensionMismatch {
        /// The offending unit text.
        unit: String,
    },
    /// A fractional or feet-inches display needs a length unit.
    #[error("fractional display `{0}` requires a length unit")]
    NotLength(String),
    /// Offset units (degC, degF) are formatted by the temperature model, not here.
    #[error("affine unit `{0}` cannot be used as a display unit here")]
    Affine(String),
    /// The value carries a leftover `pi` factor in the display unit, so it has no exact decimal.
    #[error("value is not an exact rational in display unit `{0}`")]
    Inexact(String),
    /// Exact arithmetic failed.
    #[error(transparent)]
    Magnitude(#[from] MagnitudeError),
}

/// How a value is presented: a plain unit, a unit with a fraction, or feet-inches.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DisplaySpec {
    /// Decimal in the named unit expression, e.g. `mm`.
    Unit(String),
    /// Whole units plus a fraction of `denom`, e.g. `in/32` -> `45-15/16in`.
    Fraction {
        /// Unit symbol (a length unit).
        unit: String,
        /// Denominator the value is rounded to; the fraction prints in lowest terms.
        denom: u32,
    },
    /// Feet and inches with an inch fraction of `denom`, e.g. `3'9-15/16"`.
    FeetInches {
        /// Inch-fraction denominator.
        denom: u32,
    },
}

impl DisplaySpec {
    /// Parse the text inside `#[display(...)]`, `#![display(...)]` or `[units] display`.
    pub fn parse(text: &str) -> Result<Self, DisplayError> {
        let t = text.trim();
        if t.is_empty() {
            return Err(DisplayError::Invalid(text.to_string()));
        }
        // A trailing `/<digits>` is a fraction; `mm/rev` is a unit quotient.
        let (head, denom) = match t.rsplit_once('/') {
            Some((h, d)) if !d.is_empty() && d.bytes().all(|b| b.is_ascii_digit()) => {
                let n: u32 = d
                    .parse()
                    .map_err(|_| DisplayError::Invalid(text.to_string()))?;
                if n == 0 {
                    return Err(DisplayError::ZeroDenominator(text.to_string()));
                }
                (h.trim(), Some(n))
            }
            _ => (t, None),
        };
        if head.is_empty() {
            return Err(DisplayError::Invalid(text.to_string()));
        }
        let spec = match (head, denom) {
            ("ft_in", d) => Self::FeetInches {
                denom: d.unwrap_or(FT_IN_DEFAULT_DENOM),
            },
            (unit, Some(denom)) => Self::Fraction {
                unit: unit.to_string(),
                denom,
            },
            (unit, None) => Self::Unit(unit.to_string()),
        };
        trace!(text, ?spec, "parsed display spec");
        Ok(spec)
    }
}

/// The four places a display choice can come from, highest priority first.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DisplayChoices {
    /// `#[display(...)]` on the parameter.
    pub parameter: Option<DisplaySpec>,
    /// `#![display(...)]` on the nearest enclosing scope that has one.
    pub scope: Option<DisplaySpec>,
    /// `[units] display` in `cadetta.toml` (already looked up for the value's kind).
    pub config: Option<DisplaySpec>,
    /// The unit the value was authored in.
    pub authored: Option<DisplaySpec>,
}

impl DisplayChoices {
    /// Pick parameter, then scope, then config, then the authored unit (quantities.md 6).
    pub fn resolve(&self) -> Option<&DisplaySpec> {
        let picked = [
            ("parameter", &self.parameter),
            ("scope", &self.scope),
            ("config", &self.config),
            ("authored", &self.authored),
        ]
        .into_iter()
        .find_map(|(origin, s)| s.as_ref().map(|s| (origin, s)));
        if let Some((origin, spec)) = picked {
            debug!(origin, ?spec, "display spec selected");
            Some(spec)
        } else {
            debug!("no display spec at any level");
            None
        }
    }
}

/// Render `value` (exact, SI storage) by the highest-priority spec in `choices`.
///
/// Returns `Ok(None)` when no level supplies a spec.
pub fn format_with(
    table: &UnitTable,
    value: &Magnitude,
    dim: &Dim,
    choices: &DisplayChoices,
) -> Result<Option<String>, DisplayError> {
    match choices.resolve() {
        Some(spec) => format_value(table, value, dim, spec).map(Some),
        None => Ok(None),
    }
}

/// Render `value` (exact, SI storage) in the given display spec.
pub fn format_value(
    table: &UnitTable,
    value: &Magnitude,
    dim: &Dim,
    spec: &DisplaySpec,
) -> Result<String, DisplayError> {
    match spec {
        DisplaySpec::Unit(u) => {
            let (r, minor) = in_unit(table, value, dim, u)?;
            Ok(format!("{}{u}", decimal(&r, minor)))
        }
        DisplaySpec::Fraction { unit, denom } => {
            check_length(table, unit, spec)?;
            let (r, _) = in_unit(table, value, dim, unit)?;
            let (neg, units, num, den) = split_fraction(&r, *denom);
            Ok(format!("{}{unit}", mixed(neg, &units, &num, &den)))
        }
        DisplaySpec::FeetInches { denom } => {
            check_length(table, "in", spec)?;
            let (inches, _) = in_unit(table, value, dim, "in")?;
            // Round once, in units of 1/denom inch, so carries propagate to feet.
            let ticks = round_half_away(&inches.mul(&Rational::from_int(*denom)));
            let neg = ticks.is_negative();
            let ticks = ticks.abs();
            let per_foot = BigInt::from(12) * BigInt::from(*denom);
            let (feet, rem) = ticks.div_rem(&per_foot);
            let (inch, tick) = rem.div_rem(&BigInt::from(*denom));
            let (num, den) = lowest(&tick, *denom);
            let sign = if neg { "-" } else { "" };
            let frac = if num.is_zero() {
                String::new()
            } else {
                format!("-{num}/{den}")
            };
            Ok(format!("{sign}{feet}'{inch}{frac}\""))
        }
    }
}

/// `45-15/16`, `2`, or `-1/2` (no leading zero whole before a bare fraction).
fn mixed(neg: bool, whole: &BigInt, num: &BigInt, den: &BigInt) -> String {
    let sign = if neg { "-" } else { "" };
    match (whole.is_zero(), num.is_zero()) {
        (_, true) => format!("{}{whole}", if whole.is_zero() { "" } else { sign }),
        (true, false) => format!("{sign}{num}/{den}"),
        (false, false) => format!("{sign}{whole}-{num}/{den}"),
    }
}

/// Convert an SI `value` to the unit `u`; also returns the fixed decimals that unit imposes.
fn in_unit(
    table: &UnitTable,
    value: &Magnitude,
    dim: &Dim,
    u: &str,
) -> Result<(Rational, Option<u32>), DisplayError> {
    let resolved = table.parse(u)?;
    if table.lookup(u).is_some_and(|d| d.affine) {
        return Err(DisplayError::Affine(u.to_string()));
    }
    // Only dimension is compared; kind compatibility is the kind table's job.
    if &resolved.dim != dim {
        return Err(DisplayError::DimensionMismatch {
            unit: u.to_string(),
        });
    }
    let q = value.div(&resolved.factor)?;
    if q.pi_exp() != 0 && !q.is_zero() {
        return Err(DisplayError::Inexact(u.to_string()));
    }
    let minor = fixed_decimals(dim, u);
    Ok((q.rational().clone(), minor))
}

/// Fixed precision for money (currency minor unit) and percentages (one decimal).
fn fixed_decimals(dim: &Dim, unit: &str) -> Option<u32> {
    if unit == "%" {
        return Some(1);
    }
    if is_currency_code(unit) && dim.dimension.currency_exp(unit) == 1 {
        return Some(minor_digits(unit));
    }
    None
}

fn is_currency_code(s: &str) -> bool {
    s.len() == 3 && s.bytes().all(|b| b.is_ascii_uppercase())
}

/// ISO 4217 minor-unit digits; two unless the currency is one of the listed exceptions.
pub fn minor_digits(code: &str) -> u32 {
    match code {
        "JPY" | "KRW" | "VND" | "CLP" | "ISK" | "UGX" | "PYG" | "RWF" | "XAF" | "XOF" => 0,
        "KWD" | "BHD" | "OMR" | "JOD" | "TND" | "IQD" | "LYD" => 3,
        _ => 2,
    }
}

fn check_length(table: &UnitTable, unit: &str, spec: &DisplaySpec) -> Result<(), DisplayError> {
    let r = table.parse(unit)?;
    let len = table.parse("m")?;
    if r.dim == len.dim {
        Ok(())
    } else {
        Err(DisplayError::NotLength(format!("{spec:?}")))
    }
}

/// Round to the nearest integer, ties away from zero.
fn round_half_away(r: &Rational) -> BigInt {
    let two = BigInt::from(2);
    let (n, d) = (r.numer().abs(), r.denom().clone());
    let q = (n * &two + &d).div_floor(&(d * &two));
    if r.numer().is_negative() { -q } else { q }
}

/// `tick/denom` reduced to lowest terms.
fn lowest(tick: &BigInt, denom: u32) -> (BigInt, BigInt) {
    let d = BigInt::from(denom);
    let g = tick.gcd(&d);
    if g.is_zero() {
        return (BigInt::zero(), BigInt::one());
    }
    (tick / &g, d / g)
}

/// Round `r` to `1/denom`; returns (negative, whole, fraction numerator, fraction denominator).
fn split_fraction(r: &Rational, denom: u32) -> (bool, BigInt, BigInt, BigInt) {
    let ticks = round_half_away(&r.mul(&Rational::from_int(denom)));
    let neg = ticks.is_negative();
    let (whole, tick) = ticks.abs().div_rem(&BigInt::from(denom));
    let (n, d) = lowest(&tick, denom);
    (neg, whole, n, d)
}

/// Decimal text of `r`: fixed places when `fixed` is set, else exact when it terminates
/// (otherwise rounded to 6 places), trailing zeros trimmed; always half away from zero.
fn decimal(r: &Rational, fixed: Option<u32>) -> String {
    let places = fixed.unwrap_or_else(|| exact_places(r).unwrap_or(DEFAULT_DECIMALS));
    let scale = BigInt::from(10).pow(places);
    let scaled = round_half_away(&r.mul(&Rational::from_int(scale.clone())));
    let neg = scaled.is_negative();
    let (int, frac) = scaled.abs().div_rem(&scale);
    let mut out = String::new();
    if neg {
        out.push('-');
    }
    out.push_str(&int.to_string());
    if places > 0 {
        let mut f = format!("{:0>w$}", frac.to_string(), w = places as usize);
        if fixed.is_none() {
            while f.ends_with('0') {
                f.pop();
            }
        }
        if !f.is_empty() {
            out.push('.');
            out.push_str(&f);
        }
    }
    // A value that rounds to zero prints without a sign.
    if out.starts_with('-') && out[1..].bytes().all(|b| b == b'0' || b == b'.') {
        out.remove(0);
    }
    out
}

/// Decimal places needed to write `r` exactly, if its denominator is `2^a * 5^b` and small.
fn exact_places(r: &Rational) -> Option<u32> {
    let mut d = r.denom().clone();
    let (mut twos, mut fives) = (0u32, 0u32);
    let (two, five) = (BigInt::from(2), BigInt::from(5));
    while (&d % &two).is_zero() {
        d /= &two;
        twos += 1;
    }
    while (&d % &five).is_zero() {
        d /= &five;
        fives += 1;
    }
    let places = twos.max(fives);
    (d.is_one() && places <= 30).then_some(places)
}

/// Print a unitless ratio (`0.125`) as a percentage with one decimal: `12.5%`.
pub fn format_percent(ratio: &Rational) -> String {
    let pct = ratio.mul(&Rational::from_int(100));
    format!("{}%", decimal(&pct, Some(1)))
}

/// Print `amount` of currency `code` at the currency's minor unit, e.g. `12.50USD`.
pub fn format_money(amount: &Rational, code: &str) -> String {
    format!("{}{code}", decimal(amount, Some(minor_digits(code))))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t() -> UnitTable {
        UnitTable::builtin()
    }

    /// Magnitude of `text` inches in SI.
    fn inches(text: &str) -> Magnitude {
        let f = t().parse("in").unwrap().factor;
        Magnitude::new(Rational::parse_decimal(text).unwrap(), 0).mul(&f)
    }

    /// Dimension of the unit expression `u`.
    fn dim(u: &str) -> Dim {
        t().parse(u).unwrap().dim
    }

    fn spec(s: &str) -> DisplaySpec {
        DisplaySpec::parse(s).unwrap()
    }

    // frob:tests crates/cadetta-units/src/display.rs::tests.precedence_order
    #[test]
    fn precedence_order() {
        let mut c = DisplayChoices {
            parameter: Some(spec("mm")),
            scope: Some(spec("cm")),
            config: Some(spec("in")),
            authored: Some(spec("ft")),
        };
        assert_eq!(c.resolve(), Some(&spec("mm")));
        c.parameter = None;
        assert_eq!(c.resolve(), Some(&spec("cm")));
        c.scope = None;
        assert_eq!(c.resolve(), Some(&spec("in")));
        c.config = None;
        assert_eq!(c.resolve(), Some(&spec("ft")));
        c.authored = None;
        assert_eq!(c.resolve(), None);
        let c = DisplayChoices {
            scope: Some(spec("mm")),
            authored: Some(spec("in")),
            ..Default::default()
        };
        let v = inches("1");
        assert_eq!(
            format_with(&t(), &v, &dim("m"), &c).unwrap().unwrap(),
            "25.4mm"
        );
    }

    // frob:tests crates/cadetta-units/src/display.rs::tests.inch_fraction
    #[test]
    fn inch_fraction() {
        let v = inches("45.9375");
        assert_eq!(
            format_value(&t(), &v, &dim("m"), &spec("in/32")).unwrap(),
            "45-15/16in"
        );
        assert_eq!(
            format_value(&t(), &inches("2"), &dim("m"), &spec("in/32")).unwrap(),
            "2in"
        );
        assert_eq!(
            format_value(&t(), &inches("-0.5"), &dim("m"), &spec("in/8")).unwrap(),
            "-1/2in"
        );
        // rounds half away from zero to 1/4
        assert_eq!(
            format_value(&t(), &inches("0.125"), &dim("m"), &spec("in/4")).unwrap(),
            "1/4in"
        );
        assert_eq!(
            format_value(&t(), &inches("0.99"), &dim("m"), &spec("in/4")).unwrap(),
            "1in"
        );
    }

    // frob:tests crates/cadetta-units/src/display.rs::tests.feet_inches
    #[test]
    fn feet_inches() {
        let v = inches("45.9375");
        assert_eq!(
            format_value(&t(), &v, &dim("m"), &spec("ft_in")).unwrap(),
            "3'9-15/16\""
        );
        assert_eq!(
            format_value(&t(), &inches("36"), &dim("m"), &spec("ft_in")).unwrap(),
            "3'0\""
        );
        // carry from inches into feet
        assert_eq!(
            format_value(&t(), &inches("11.99"), &dim("m"), &spec("ft_in")).unwrap(),
            "1'0\""
        );
        assert_eq!(
            format_value(&t(), &inches("45.9375"), &dim("m"), &spec("ft_in/4")).unwrap(),
            "3'10\""
        );
        assert!(matches!(
            format_value(&t(), &inches("1"), &dim("kg"), &spec("ft_in")),
            Err(DisplayError::DimensionMismatch { .. })
        ));
    }

    // frob:tests crates/cadetta-units/src/display.rs::tests.spec_parsing
    #[test]
    fn spec_parsing() {
        assert_eq!(spec("mm/rev"), DisplaySpec::Unit("mm/rev".into()));
        assert_eq!(spec("ft_in"), DisplaySpec::FeetInches { denom: 16 });
        assert!(matches!(
            DisplaySpec::parse("in/0"),
            Err(DisplayError::ZeroDenominator(_))
        ));
        assert!(matches!(
            DisplaySpec::parse(" "),
            Err(DisplayError::Invalid(_))
        ));
    }

    // frob:tests crates/cadetta-units/src/display.rs::tests.plain_units
    #[test]
    fn plain_units() {
        assert_eq!(
            format_value(&t(), &inches("0.5"), &dim("m"), &spec("mm")).unwrap(),
            "12.7mm"
        );
        assert_eq!(
            format_value(&t(), &inches("1"), &dim("m"), &spec("in")).unwrap(),
            "1in"
        );
        assert!(matches!(
            format_value(&t(), &inches("1"), &dim("m"), &spec("nope")),
            Err(DisplayError::Unit(_))
        ));
        let third = Magnitude::new(Rational::new(1, 3).unwrap(), 0);
        assert_eq!(
            format_value(&t(), &third, &dim("m"), &spec("m")).unwrap(),
            "0.333333m"
        );
    }

    // frob:tests crates/cadetta-units/src/display.rs::tests.money_and_percent
    #[test]
    fn money_and_percent() {
        let usd = Magnitude::new(Rational::parse_decimal("12.345").unwrap(), 0);
        assert_eq!(
            format_value(&t(), &usd, &dim("USD"), &spec("USD")).unwrap(),
            "12.35USD"
        );
        let neg = Magnitude::new(Rational::parse_decimal("-12.345").unwrap(), 0);
        assert_eq!(
            format_value(&t(), &neg, &dim("USD"), &spec("USD")).unwrap(),
            "-12.35USD"
        );
        let yen = Magnitude::new(Rational::parse_decimal("99.5").unwrap(), 0);
        assert_eq!(
            format_value(&t(), &yen, &dim("JPY"), &spec("JPY")).unwrap(),
            "100JPY"
        );
        assert_eq!(
            format_money(&Rational::parse_decimal("3").unwrap(), "USD"),
            "3.00USD"
        );
        let half = Magnitude::new(Rational::new(1, 8).unwrap(), 0);
        assert_eq!(
            format_value(&t(), &half, &dim("%"), &spec("%")).unwrap(),
            "12.5%"
        );
        assert_eq!(
            format_percent(&Rational::parse_decimal("0.12345").unwrap()),
            "12.3%"
        );
        assert_eq!(
            format_percent(&Rational::parse_decimal("0.12350").unwrap()),
            "12.4%"
        );
        assert_eq!(
            format_percent(&Rational::parse_decimal("-0.0005").unwrap()),
            "-0.1%"
        );
        assert_eq!(
            format_percent(&Rational::parse_decimal("-0.0004").unwrap()),
            "0.0%"
        );
    }
}
