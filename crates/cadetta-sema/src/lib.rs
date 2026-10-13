//! Name resolution, kind checking and evaluation into design IR.
//!
//! Specification: `docs/spec/language.md`.

pub mod affine;
pub mod attrs;
pub mod cite;
pub mod deps;
pub mod eval;
pub mod fits;
pub mod imports;
pub mod kindcheck;
pub mod modules;
pub mod process;
pub mod resolve;
pub mod tables;
