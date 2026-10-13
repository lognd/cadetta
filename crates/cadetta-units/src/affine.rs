// frob:ticket 01M4SPWNEF5D7VN10KTWKPM465
//! Affine temperature units, absolute Temperature and TemperatureDelta.
//!
//! Specification: `docs/spec/quantities.md` section 3. Absolute units
//! (`K`, `degC`, `degF`) carry an exact offset to kelvin; delta units
//! (`dK`, `ddegC`, `ddegF`) carry none. Everything is exact rational.
//!
//! SEAM: the built-in unit table (ticket GNSP35P, `units.rs`) is not
//! merged yet, so the six temperature units are defined locally in
//! [`TemperatureUnit`]. When the table lands it should delegate here.

use thiserror::Error;
use tracing::{debug, trace};

use crate::rational::{MagnitudeError, Rational};

/// Errors from affine temperature handling.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum AffineError {
    /// An affine (offset) unit was used in a product or quotient.
    #[error("affine unit `{0}` cannot be multiplied or divided")]
    AffineMultiply(&'static str),
    /// The unit name is not a temperature unit.
    #[error("`{0}` is not a temperature unit")]
    UnknownUnit(String),
    /// A delta unit was used where an absolute one is required, or vice versa.
    #[error("unit `{0}` has the wrong temperature class for this operation")]
    WrongClass(&'static str),
    /// Exact arithmetic failed.
    #[error(transparent)]
    Magnitude(#[from] MagnitudeError),
}

/// Whether a temperature unit is an absolute point or a difference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TempClass {
    /// Absolute temperature (`K`, `degC`, `degF`); `K` has zero offset.
    Absolute,
    /// Temperature difference (`dK`, `ddegC`, `ddegF`); never offset.
    Delta,
}

/// One of the six built-in temperature units.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TemperatureUnit {
    /// Kelvin.
    K,
    /// Degree Celsius (affine).
    DegC,
    /// Degree Fahrenheit (affine).
    DegF,
    /// Kelvin difference.
    DK,
    /// Celsius difference.
    DDegC,
    /// Fahrenheit difference.
    DDegF,
}

impl TemperatureUnit {
    /// Parse a unit atom by its source spelling.
    pub fn parse(name: &str) -> Result<Self, AffineError> {
        let u = match name {
            "K" => Self::K,
            "degC" => Self::DegC,
            "degF" => Self::DegF,
            "dK" => Self::DK,
            "ddegC" => Self::DDegC,
            "ddegF" => Self::DDegF,
            other => {
                debug!(unit = other, "not a temperature unit");
                return Err(AffineError::UnknownUnit(other.to_owned()));
            }
        };
        trace!(unit = name, "parsed temperature unit");
        Ok(u)
    }

    /// Source spelling of the unit.
    pub fn name(self) -> &'static str {
        match self {
            Self::K => "K",
            Self::DegC => "degC",
            Self::DegF => "degF",
            Self::DK => "dK",
            Self::DDegC => "ddegC",
            Self::DDegF => "ddegF",
        }
    }

    /// Absolute or delta class.
    pub fn class(self) -> TempClass {
        match self {
            Self::K | Self::DegC | Self::DegF => TempClass::Absolute,
            Self::DK | Self::DDegC | Self::DDegF => TempClass::Delta,
        }
    }

    /// True for units with a nonzero offset (`degC`, `degF`).
    pub fn is_affine(self) -> bool {
        matches!(self, Self::DegC | Self::DegF)
    }

    /// Exact kelvin per one unit step.
    pub fn factor(self) -> Rational {
        match self {
            Self::K | Self::DegC | Self::DK | Self::DDegC => Rational::one(),
            Self::DegF | Self::DDegF => ratio(5, 9),
        }
    }

    /// Exact kelvin value of zero in this unit (zero for all but affine ones).
    pub fn offset(self) -> Rational {
        match self {
            // 273.15 K
            Self::DegC => ratio(27315, 100),
            // 459.67 degF above absolute zero, times 5/9
            Self::DegF => ratio(45967, 100).mul(&ratio(5, 9)),
            _ => Rational::from_int(0),
        }
    }

    /// Refuse a product or quotient involving an affine unit.
    pub fn check_multiply(self) -> Result<(), AffineError> {
        if self.is_affine() {
            debug!(unit = self.name(), "rejected multiply of affine unit");
            Err(AffineError::AffineMultiply(self.name()))
        } else {
            Ok(())
        }
    }
}

fn ratio(n: i64, d: i64) -> Rational {
    Rational::new(n, d).expect("nonzero literal denominator")
}

/// An absolute temperature, stored exactly in kelvin.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Temperature {
    kelvin: Rational,
}

/// A temperature difference, stored exactly in kelvin.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TemperatureDelta {
    kelvin: Rational,
}

impl Temperature {
    /// Build from a value in an absolute unit, applying the exact offset.
    pub fn new(value: &Rational, unit: TemperatureUnit) -> Result<Self, AffineError> {
        if unit.class() != TempClass::Absolute {
            return Err(AffineError::WrongClass(unit.name()));
        }
        let kelvin = value.mul(&unit.factor()).add(&unit.offset());
        trace!(unit = unit.name(), %kelvin, "temperature to kelvin");
        Ok(Self { kelvin })
    }

    /// Exact value in kelvin.
    pub fn kelvin(&self) -> &Rational {
        &self.kelvin
    }

    /// Exact value expressed in an absolute unit.
    pub fn to_unit(&self, unit: TemperatureUnit) -> Result<Rational, AffineError> {
        if unit.class() != TempClass::Absolute {
            return Err(AffineError::WrongClass(unit.name()));
        }
        Ok(self.kelvin.sub(&unit.offset()).div(&unit.factor())?)
    }

    /// Difference of two temperatures is a [`TemperatureDelta`].
    pub fn sub(&self, other: &Self) -> TemperatureDelta {
        TemperatureDelta {
            kelvin: self.kelvin.sub(&other.kelvin),
        }
    }

    /// Shift by a delta, giving a temperature.
    pub fn add_delta(&self, d: &TemperatureDelta) -> Self {
        Self {
            kelvin: self.kelvin.add(&d.kelvin),
        }
    }
}

impl TemperatureDelta {
    /// Build from a value in a delta unit (no offset).
    pub fn new(value: &Rational, unit: TemperatureUnit) -> Result<Self, AffineError> {
        if unit.class() != TempClass::Delta {
            return Err(AffineError::WrongClass(unit.name()));
        }
        Ok(Self {
            kelvin: value.mul(&unit.factor()),
        })
    }

    /// Exact value in kelvin steps.
    pub fn kelvin(&self) -> &Rational {
        &self.kelvin
    }

    /// Exact value expressed in a delta unit.
    pub fn to_unit(&self, unit: TemperatureUnit) -> Result<Rational, AffineError> {
        if unit.class() != TempClass::Delta {
            return Err(AffineError::WrongClass(unit.name()));
        }
        Ok(self.kelvin.div(&unit.factor())?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn int(n: i64) -> Rational {
        Rational::from_int(n)
    }

    // frob:tests crates/cadetta-units/src/affine.rs::Temperature.sub kind=unit
    #[test]
    fn temperature_difference_is_delta() {
        let a = Temperature::new(&int(30), TemperatureUnit::DegC).unwrap();
        let b = Temperature::new(&int(20), TemperatureUnit::DegC).unwrap();
        let d: TemperatureDelta = a.sub(&b);
        assert_eq!(d.to_unit(TemperatureUnit::DDegC).unwrap(), int(10));
        assert_eq!(d.to_unit(TemperatureUnit::DDegF).unwrap(), int(18));
        assert_eq!(b.add_delta(&d), a);
    }

    // frob:tests crates/cadetta-units/src/affine.rs::TemperatureUnit.check_multiply kind=unit
    #[test]
    fn affine_units_reject_multiply() {
        assert_eq!(
            TemperatureUnit::DegC.check_multiply(),
            Err(AffineError::AffineMultiply("degC"))
        );
        assert!(TemperatureUnit::DegF.check_multiply().is_err());
        assert!(TemperatureUnit::K.check_multiply().is_ok());
        assert!(TemperatureUnit::DDegC.check_multiply().is_ok());
    }

    // frob:tests crates/cadetta-units/src/affine.rs::Temperature.new kind=unit
    #[test]
    fn offsets_apply_exactly() {
        let f = Temperature::new(&int(32), TemperatureUnit::DegF).unwrap();
        assert_eq!(f.kelvin(), &Rational::new(27315, 100).unwrap());
        let boil = Temperature::new(&int(212), TemperatureUnit::DegF).unwrap();
        assert_eq!(boil.kelvin(), &Rational::new(37315, 100).unwrap());
        let c = Temperature::new(&int(100), TemperatureUnit::DegC).unwrap();
        assert_eq!(c, boil);
        assert_eq!(c.to_unit(TemperatureUnit::DegF).unwrap(), int(212));
        let zero = Temperature::new(&int(0), TemperatureUnit::K).unwrap();
        assert_eq!(
            zero.to_unit(TemperatureUnit::DegF).unwrap(),
            Rational::new(-45967, 100).unwrap()
        );
    }

    // frob:tests crates/cadetta-units/src/affine.rs::TemperatureUnit.parse kind=unit
    #[test]
    fn delta_units_parse_without_offset() {
        for n in ["dK", "ddegC", "ddegF"] {
            let u = TemperatureUnit::parse(n).unwrap();
            assert_eq!(u.class(), TempClass::Delta);
            assert!(u.offset().is_zero());
            assert!(!u.is_affine());
        }
        let d = TemperatureDelta::new(&int(9), TemperatureUnit::DDegF).unwrap();
        assert_eq!(d.kelvin(), &int(5));
        assert!(TemperatureDelta::new(&int(1), TemperatureUnit::K).is_err());
        assert!(Temperature::new(&int(1), TemperatureUnit::DK).is_err());
        assert!(matches!(
            TemperatureUnit::parse("mm"),
            Err(AffineError::UnknownUnit(_))
        ));
    }
}
