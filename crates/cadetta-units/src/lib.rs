//! Exact rational quantities, units, kinds and tolerances.
//!
//! Specification: `docs/spec/quantities.md`.

pub mod affine;
pub mod compare;
pub mod dimension;
pub mod display;
pub mod inexact;
pub mod kind;
pub mod rational;
pub mod square;
pub mod tolerance;
pub mod units;

pub use affine::{AffineError, TempClass, Temperature, TemperatureDelta, TemperatureUnit};
pub use compare::{CmpOp, Comparison, Note, Operand, Verdict, compare, compare_nominals};
pub use dimension::{Dim, Dimension, DimensionError, SI_BASE_COUNT};
pub use display::{DisplayChoices, DisplayError, DisplaySpec, format_value, format_with};
pub use inexact::{InexactError, InexactValue, RootOp, Value};
pub use kind::{Assigned, KindError, KindTable, KindWarning, KindedDim, Rounding};
pub use rational::{Magnitude, MagnitudeError, Rational, Sum};
pub use tolerance::{Affine, InexactInterval, ToleranceError, Toleranced, VarId};
pub use units::{ResolvedUnit, UnitDef, UnitError, UnitKind, UnitTable};
