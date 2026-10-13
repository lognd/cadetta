//! Exact rational quantities, units, kinds and tolerances.
//!
//! Specification: `docs/spec/quantities.md`.

pub mod dimension;
pub mod rational;
pub mod units;

pub use dimension::{Dim, Dimension, DimensionError, SI_BASE_COUNT};
pub use rational::{Magnitude, MagnitudeError, Rational, Sum};
pub use units::{ResolvedUnit, UnitDef, UnitError, UnitKind, UnitTable};
