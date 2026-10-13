//! Kind policy (`[obligations.kinds.<kind>] required_for`) and discharge rules.
//!
//! Specification: `docs/spec/obligations.md` 2.3 and 4; `docs/spec/config-cli.md` 2.
//! frob:ticket 01M4ST8EMQTXAGZAWGE2M5A4BR

use std::collections::BTreeMap;

use thiserror::Error;
use tracing::{debug, trace};

use crate::graph::{ObligationKind, ObligationState};

/// Reason attached to a `sync` obligation that only `sw-equations` touched.
pub const UNVERIFIABLE_REASON: &str = "unverifiable without drive";

/// When a kind gates `cadetta check` (obligations.md 4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RequiredFor {
    /// Gates `cadetta check`.
    Check,
    /// Gates `cadetta check --release` only (and, being a superset, nothing earlier).
    Release,
}

impl RequiredFor {
    /// Parse the config value (`check` or `release`).
    pub fn parse(s: &str) -> Result<Self, KindsError> {
        match s {
            "check" => Ok(Self::Check),
            "release" => Ok(Self::Release),
            other => Err(KindsError::BadRequiredFor(other.to_owned())),
        }
    }
}

/// Errors from kind policy handling.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum KindsError {
    /// `required_for` was neither `check` nor `release`.
    #[error("invalid required_for `{0}` (expected `check` or `release`)")]
    BadRequiredFor(String),
    /// Only `sync` and `solver` accept a kind-level `required_for`.
    #[error("[obligations.kinds.{0}] is not configurable: only `sync` and `solver` are")]
    NotConfigurable(String),
}

/// What performed an action on an obligation, for the discharge rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Actor {
    /// `cadetta sync` driving a real CAD tool.
    Sync,
    /// `cadetta solve`.
    Solve,
    /// Emission of a file by an adapter that cannot read the CAD tool back (`sw-equations`).
    EquationsEmission,
}

/// Outcome of applying an [`Actor`] to an obligation kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Discharge {
    /// The obligation becomes discharged at current digests.
    Discharged,
    /// The state is unchanged; `reason` says why.
    Unchanged {
        /// Human-readable reason, reported with O004.
        reason: &'static str,
    },
}

/// Decide whether `actor` discharges an obligation of `kind` (obligations.md 2.3).
///
/// `sw-equations` emission never discharges `sync`; the state stays `open`.
pub fn discharge(kind: ObligationKind, actor: Actor) -> Discharge {
    let out = match (kind, actor) {
        (ObligationKind::Sync, Actor::Sync) | (ObligationKind::Solver, Actor::Solve) => {
            Discharge::Discharged
        }
        (ObligationKind::Sync, Actor::EquationsEmission) => Discharge::Unchanged {
            reason: UNVERIFIABLE_REASON,
        },
        _ => Discharge::Unchanged {
            reason: "actor does not discharge this kind",
        },
    };
    debug!(kind = kind.name(), ?actor, ?out, "discharge decision");
    out
}

/// Parsed `[obligations.kinds.*]` table: which kinds gate, and when.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct KindPolicy {
    gates: BTreeMap<ObligationKind, RequiredFor>,
}

impl KindPolicy {
    /// An empty policy: nothing gates.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set `required_for` for a kind by config name; only `sync` and `solver` are accepted.
    pub fn set(&mut self, kind: &str, required_for: &str) -> Result<(), KindsError> {
        let k = match ObligationKind::from_name(kind) {
            Some(k @ (ObligationKind::Sync | ObligationKind::Solver)) => k,
            _ => return Err(KindsError::NotConfigurable(kind.to_owned())),
        };
        let rf = RequiredFor::parse(required_for)?;
        debug!(kind = k.name(), ?rf, "kind policy set");
        self.gates.insert(k, rf);
        Ok(())
    }

    /// The configured `required_for` of a kind, if any.
    pub fn required_for(&self, kind: ObligationKind) -> Option<RequiredFor> {
        self.gates.get(&kind).copied()
    }

    /// Whether an obligation in `state` fails the run (`release` = `check --release`).
    ///
    /// Only stale or open obligations of a configured kind gate; unset means not gating.
    pub fn gates(&self, kind: ObligationKind, state: ObligationState, release: bool) -> bool {
        let pending = matches!(state, ObligationState::Stale | ObligationState::Open);
        let gate = match self.required_for(kind) {
            Some(RequiredFor::Check) => true,
            Some(RequiredFor::Release) => release,
            None => false,
        };
        trace!(
            kind = kind.name(),
            ?state,
            release,
            pending,
            gate,
            "gate check"
        );
        pending && gate
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // frob:tests crates/cadetta-obligations/src/kinds.rs::KindPolicy.gates
    #[test]
    fn required_for_gates_sync_and_solver() {
        let mut p = KindPolicy::new();
        p.set("sync", "release").unwrap();
        p.set("solver", "check").unwrap();
        let (sy, so) = (ObligationKind::Sync, ObligationKind::Solver);
        assert!(!p.gates(sy, ObligationState::Open, false));
        assert!(p.gates(sy, ObligationState::Open, true));
        assert!(p.gates(sy, ObligationState::Stale, true));
        assert!(!p.gates(sy, ObligationState::Discharged, true));
        assert!(p.gates(so, ObligationState::Stale, false));
        // unset: not gating
        assert!(!KindPolicy::new().gates(sy, ObligationState::Open, true));
    }

    // frob:tests crates/cadetta-obligations/src/kinds.rs::discharge
    #[test]
    fn equations_emission_never_discharges_sync() {
        assert_eq!(
            discharge(ObligationKind::Sync, Actor::EquationsEmission),
            Discharge::Unchanged {
                reason: UNVERIFIABLE_REASON
            }
        );
        assert_eq!(
            discharge(ObligationKind::Sync, Actor::Sync),
            Discharge::Discharged
        );
        assert_eq!(
            discharge(ObligationKind::Solver, Actor::EquationsEmission),
            Discharge::Unchanged {
                reason: "actor does not discharge this kind"
            }
        );
    }

    // frob:tests crates/cadetta-obligations/src/kinds.rs::KindPolicy.set
    #[test]
    fn set_rejects_bad_input() {
        let mut p = KindPolicy::new();
        assert_eq!(
            p.set("fit", "check"),
            Err(KindsError::NotConfigurable("fit".into()))
        );
        assert_eq!(
            p.set("sync", "never"),
            Err(KindsError::BadRequiredFor("never".into()))
        );
        assert_eq!(p.required_for(ObligationKind::Sync), None);
    }
}
