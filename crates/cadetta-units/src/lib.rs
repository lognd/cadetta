//! Exact rational quantities, units, kinds and tolerances.
//!
//! Specification: `docs/spec/quantities.md`.

pub mod affine;
pub mod dimension;
pub mod rational;

pub use affine::{AffineError, TempClass, Temperature, TemperatureDelta, TemperatureUnit};
pub use dimension::{Dim, Dimension, DimensionError, SI_BASE_COUNT};
pub use rational::{Magnitude, MagnitudeError, Rational, Sum};
