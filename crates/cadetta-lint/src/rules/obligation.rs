//! Lints O001-O003: stale, open and unsigned role attestations.
//!
//! Specification: `docs/spec/lints.md` (O); `docs/spec/obligations.md` 2.3 and 4.

// frob:ticket 01M4STCCZ58C875BTMQ9E54BMT

use cadetta_obligations::graph::{ObligationId, ObligationKind, ObligationState};
use cadetta_obligations::kinds::RequiredFor;
use cadetta_obligations::roles::Policy;
use tracing::{debug, trace};

use crate::diag::{Diagnostic, Severity};

/// Rule code: a role attestation is stale (warning; error when its role gates `check`).
pub const O001: &str = "O001";
/// Rule code: a required role attestation was never given (warning; error when gating).
pub const O002: &str = "O002";
/// Rule code: a `require` that holds lacks its role sign-off (error).
pub const O003: &str = "O003";

/// True for kinds whose staleness belongs to another code (S003, S004, O004) or is recomputed.
fn reported_elsewhere(kind: ObligationKind) -> bool {
    matches!(
        kind,
        ObligationKind::Ots
            | ObligationKind::Citation
            | ObligationKind::Solver
            | ObligationKind::Sync
    )
}

/// True when a role covering `id` is `required_for = "check"`.
fn gates_check(policy: &Policy, id: &ObligationId) -> bool {
    policy
        .roles_covering(id)
        .iter()
        .any(|r| r.required_for == Some(RequiredFor::Check))
}

/// Run O001-O003 over the classified obligations; `policy` supplies the role gating.
pub fn check(obligations: &[(ObligationId, ObligationState)], policy: &Policy) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for (id, state) in obligations {
        let pending = matches!(state, ObligationState::Stale | ObligationState::Open);
        if !pending || reported_elsewhere(id.kind()) {
            trace!(%id, ?state, "O: not applicable");
            continue;
        }
        let stale = *state == ObligationState::Stale;
        let (code, severity, message) = if id.kind() == ObligationKind::Require {
            let what = if stale { "is stale" } else { "was never given" };
            (
                O003,
                Severity::Error,
                format!("`{id}` holds but its sign-off {what}"),
            )
        } else {
            let gating = gates_check(policy, id);
            let severity = if gating {
                Severity::Error
            } else {
                Severity::Warning
            };
            if stale {
                (
                    O001,
                    severity,
                    format!("attestation of `{id}` is stale: something it covers changed"),
                )
            } else {
                (
                    O002,
                    severity,
                    format!("`{id}` requires an attestation and has none"),
                )
            }
        };
        debug!(%id, code, ?severity, "obligation diagnostic");
        let mut d = Diagnostic::new(code, severity, message);
        d.path = Some(id.path().to_owned());
        out.push(d);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(s: &str) -> ObligationId {
        ObligationId::parse(s).unwrap()
    }

    fn policy(required_for: Option<&str>) -> Policy {
        let mut p = Policy::new();
        p.add_role("rev", &["check:**", "waiver:**"], required_for, None, false)
            .unwrap();
        p
    }

    fn run(i: &str, s: ObligationState, p: &Policy) -> Vec<Diagnostic> {
        check(&[(id(i), s)], p)
    }

    // frob:tests crates/cadetta-lint/src/rules/obligation.rs::check
    #[test]
    fn o001_warns_when_stale() {
        let d = run("check:a::X.c", ObligationState::Stale, &policy(None));
        assert_eq!(d.len(), 1);
        assert_eq!(
            (d[0].code.as_str(), d[0].severity),
            ("O001", Severity::Warning)
        );
    }

    // frob:tests crates/cadetta-lint/src/rules/obligation.rs::check
    #[test]
    fn o001_errors_for_role_required_for_check() {
        let d = run(
            "check:a::X.c",
            ObligationState::Stale,
            &policy(Some("check")),
        );
        assert_eq!(d[0].severity, Severity::Error);
        let d = run(
            "check:a::X.c",
            ObligationState::Stale,
            &policy(Some("release")),
        );
        assert_eq!(d[0].severity, Severity::Warning);
    }

    // frob:tests crates/cadetta-lint/src/rules/obligation.rs::check
    #[test]
    fn o002_fires_when_open_and_covers_waiver_review() {
        let d = run("check:a::X.c", ObligationState::Open, &policy(None));
        assert_eq!(
            (d[0].code.as_str(), d[0].severity),
            ("O002", Severity::Warning)
        );
        let d = run("waiver:w1", ObligationState::Open, &policy(None));
        assert_eq!(d[0].code, "O002");
        let d = run(
            "check:a::X.c",
            ObligationState::Open,
            &policy(Some("check")),
        );
        assert_eq!(d[0].severity, Severity::Error);
    }

    // frob:tests crates/cadetta-lint/src/rules/obligation.rs::check
    #[test]
    fn o003_errors_on_unsigned_require_only() {
        for s in [ObligationState::Open, ObligationState::Stale] {
            let d = run("require:a::X.r", s, &policy(None));
            assert_eq!(d.len(), 1);
            assert_eq!(
                (d[0].code.as_str(), d[0].severity),
                ("O003", Severity::Error)
            );
        }
    }

    // frob:tests crates/cadetta-lint/src/rules/obligation.rs::check
    #[test]
    fn s_kinds_are_not_double_reported() {
        for i in ["ots:a::X", "citation:a::X.c", "solver:a::X.s", "sync:a::X"] {
            assert!(run(i, ObligationState::Stale, &policy(None)).is_empty());
            assert!(run(i, ObligationState::Open, &policy(None)).is_empty());
        }
    }

    // frob:tests crates/cadetta-lint/src/rules/obligation.rs::check
    #[test]
    fn current_attestation_is_quiet() {
        for i in ["check:a::X.c", "require:a::X.r", "waiver:w1"] {
            for s in [
                ObligationState::Discharged,
                ObligationState::Waived,
                ObligationState::Violated,
                ObligationState::Undecided,
            ] {
                assert!(run(i, s, &policy(Some("check"))).is_empty());
            }
        }
    }
}
