//! The obligation graph, closure digests, staleness and cadetta.lock.
//!
//! Specification: `docs/spec/obligations.md`.

pub mod as_of;
pub mod citation;
pub mod digest_attrs;
pub mod graph;
pub mod impact_since;
pub mod kinds;
pub mod level;
pub mod lock;
pub mod lock_merge;
pub mod lock_package;
pub mod lock_tidy;
pub mod lock_verify;
pub mod pkg_closure;
pub mod require;
pub mod roles;
pub mod schema;
pub mod selector;
pub mod staleness;
pub mod swap;
pub mod waiver;
pub mod why_stale;
