//! The `require` rule: a require is never suppressed, and a waiver naming it is W003.
//!
//! Combines the verdict classification ([`classify_require`]) with the waiver check into one
//! evaluation: O003 is raised for an unsigned `holds` require whatever waivers exist, and a
//! waiver or `allow` naming the require is reported as W003 and has no effect. Specification:
//! `docs/spec/obligations.md` 4, `docs/spec/lints.md` 2.3.
//! frob:ticket 01M4SWN7RHS0FP5TFJC0WVCRM0

use tracing::{debug, warn};

use crate::graph::{ObligationState, Verdict};
use crate::staleness::{AttestState, classify_require};

/// The lint code of an unsigned or stale sign-off on a `holds` require.
pub const SIGNOFF_CODE: &str = "O003";

/// The lint code of a waiver or `allow` naming a require.
pub const WAIVER_CODE: &str = "W003";

/// Result of evaluating one `require` with any waiver requested against it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RequireOutcome {
    /// The obligation state; never [`ObligationState::Waived`].
    pub state: ObligationState,
    /// Whether O003 fires (the sign-off is open or stale).
    pub o003: bool,
    /// Whether W003 fires (a waiver or `allow` named this require).
    pub w003: bool,
}

impl RequireOutcome {
    /// The lint codes raised, O003 first then W003.
    pub fn codes(&self) -> Vec<&'static str> {
        let mut v = Vec::new();
        if self.o003 {
            v.push(SIGNOFF_CODE);
        }
        if self.w003 {
            v.push(WAIVER_CODE);
        }
        v
    }
}

/// Evaluate a `require`: classify it, then discard any waiver aimed at it (W003).
///
/// `waiver_named` is true when a `waive` or `allow` targets the require; it never changes the
/// state or suppresses O003.
pub fn evaluate_require(
    verdict: Option<Verdict>,
    signoff_configured: bool,
    attest: &AttestState,
    waiver_named: bool,
) -> RequireOutcome {
    let state = classify_require(verdict, signoff_configured, attest);
    let o003 = matches!(state, ObligationState::Open | ObligationState::Stale);
    if waiver_named {
        warn!(?state, "waiver names a require: W003, not suppressed");
    }
    debug!(?state, o003, waiver_named, "require evaluated");
    RequireOutcome {
        state,
        o003,
        w003: waiver_named,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::staleness::StaleReason;

    // frob:tests crates/cadetta-obligations/src/require.rs::evaluate_require
    #[test]
    fn o003_not_suppressed_by_waiver() {
        for att in [
            AttestState::Open,
            AttestState::Stale(StaleReason::default()),
        ] {
            let quiet = evaluate_require(Some(Verdict::Holds), true, &att, false);
            let waived = evaluate_require(Some(Verdict::Holds), true, &att, true);
            assert!(quiet.o003 && waived.o003);
            assert_eq!(quiet.state, waived.state);
            assert_ne!(waived.state, ObligationState::Waived);
        }
    }

    // frob:tests crates/cadetta-obligations/src/require.rs::evaluate_require
    #[test]
    fn waiver_naming_require_fires_w003() {
        let o = evaluate_require(Some(Verdict::Holds), true, &AttestState::Open, true);
        assert_eq!(o.codes(), [SIGNOFF_CODE, WAIVER_CODE]);
        let none = evaluate_require(Some(Verdict::Holds), true, &AttestState::Open, false);
        assert_eq!(none.codes(), [SIGNOFF_CODE]);
    }

    // frob:tests crates/cadetta-obligations/src/require.rs::evaluate_require
    #[test]
    fn attestation_signs_off_holds_but_not_violated() {
        let ok = evaluate_require(Some(Verdict::Holds), true, &AttestState::Current, true);
        assert_eq!(ok.state, ObligationState::Discharged);
        assert_eq!(ok.codes(), [WAIVER_CODE]);
        let bad = evaluate_require(Some(Verdict::Violated), true, &AttestState::Current, true);
        assert_eq!(bad.state, ObligationState::Violated);
        assert!(!bad.o003);
    }
}
