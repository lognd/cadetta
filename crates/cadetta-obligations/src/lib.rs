//! The obligation graph, closure digests, staleness and cadetta.lock.
//!
//! Specification: `docs/spec/obligations.md`.

pub mod as_of;
pub mod citation;
pub mod digest_attrs;
pub mod graph;
pub mod kinds;
pub mod level;
pub mod lock;
pub mod lock_merge;
pub mod lock_package;
pub mod lock_tidy;
pub mod lock_verify;
pub mod selector;
pub mod staleness;
