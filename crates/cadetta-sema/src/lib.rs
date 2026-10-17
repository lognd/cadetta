//! Name resolution, kind checking and evaluation into design IR.
//!
//! Specification: `docs/spec/language.md`.

pub mod affine;
pub mod attrs;
pub mod cite;
pub mod defaults;
pub mod deps;
pub mod eval;
pub mod fitderive;
pub mod fits;
pub mod imports;
pub mod kindcheck;
pub mod lower;
pub mod modules;
pub mod parts;
pub mod process;
pub mod renames;
pub mod resolve;
pub mod rollup;
pub mod solverdecl;
pub mod tables;
pub mod visibility;
#[cfg(test)]
mod warnprobe;
