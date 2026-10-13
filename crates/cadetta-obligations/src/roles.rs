//! Roles, policy and release gating (`[obligations.roles.*]`, `[obligations.kinds.*]`).
//!
//! A role covers obligations by selector, may restrict who attests (`members`), may demand a
//! reviewer distinct from the author, and may gate `check` or `check --release`. Specification:
//! `docs/spec/obligations.md` 4 and `docs/spec/config-cli.md` 2.
//! frob:ticket 01M4ST8F01H7SYVGYX5G9PCJYT

use std::collections::BTreeMap;

use thiserror::Error;
use tracing::{debug, trace, warn};

use crate::graph::{ObligationId, ObligationKind, ObligationState};
use crate::kinds::{KindPolicy, KindsError, RequiredFor};
use crate::selector::{Selector, SelectorError};
use crate::staleness::AttestState;

/// Errors from role configuration and attestation checks.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RoleError {
    /// A `covers` selector did not parse.
    #[error("role `{role}`: bad covers selector: {source}")]
    BadCovers {
        /// Role name.
        role: String,
        /// Underlying selector error.
        source: SelectorError,
    },
    /// A role has no `covers` entries.
    #[error("role `{0}` covers nothing: `covers` must list at least one selector")]
    EmptyCovers(String),
    /// `required_for` was invalid.
    #[error("role `{role}`: {source}")]
    BadRequiredFor {
        /// Role name.
        role: String,
        /// Underlying error.
        source: KindsError,
    },
    /// A role with this name already exists.
    #[error("role `{0}` is declared twice")]
    Duplicate(String),
    /// The attestation names a role that is not configured.
    #[error("unknown role `{0}`")]
    UnknownRole(String),
    /// `by` is not in the role's `members`.
    #[error("`{by}` is not a member of role `{role}`")]
    NotMember {
        /// Role name.
        role: String,
        /// Attester.
        by: String,
    },
    /// `distinct_from_author` is set and `by` is the author.
    #[error("role `{role}` requires a reviewer distinct from the author, but `{by}` is the author")]
    NotDistinct {
        /// Role name.
        role: String,
        /// Attester.
        by: String,
    },
    /// `distinct_from_author` is set and the author could not be determined.
    #[error("role `{0}` requires a distinct reviewer but the author is unknown")]
    AuthorUnknown(String),
    /// The role does not cover the obligation being attested.
    #[error("role `{role}` does not cover `{id}`")]
    NotCovered {
        /// Role name.
        role: String,
        /// Obligation id.
        id: String,
    },
    /// A `require` cannot be waived; only a current attestation signs it off.
    #[error("`{0}` cannot be waived: a require is never suppressed (W003); attest it instead")]
    RequireWaiver(String),
}

/// One configured role.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Role {
    /// Role name (the table key).
    pub name: String,
    /// Selectors of the obligations this role signs off.
    pub covers: Vec<Selector>,
    /// When unattested obligations covered by this role gate; `None` = not gating.
    pub required_for: Option<RequiredFor>,
    /// Who may attest; `None` = anyone.
    pub members: Option<Vec<String>>,
    /// Whether `by` must differ from the author of the newest covered change.
    pub distinct_from_author: bool,
}

impl Role {
    /// Whether the role covers the obligation.
    pub fn covers(&self, id: &ObligationId) -> bool {
        self.covers.iter().any(|s| s.matches(id))
    }

    /// Enforce `members` and `distinct_from_author` for an attestation by `by`.
    ///
    /// `author` is the author of the newest commit that changed any covered node.
    pub fn check_attester(&self, by: &str, author: Option<&str>) -> Result<(), RoleError> {
        if let Some(m) = &self.members
            && !m.iter().any(|x| x == by)
        {
            warn!(role = %self.name, by, "attestation rejected: not a member");
            return Err(RoleError::NotMember {
                role: self.name.clone(),
                by: by.to_owned(),
            });
        }
        if self.distinct_from_author {
            match author {
                None => {
                    warn!(role = %self.name, "attestation rejected: author unknown");
                    return Err(RoleError::AuthorUnknown(self.name.clone()));
                }
                Some(a) if a == by => {
                    warn!(role = %self.name, by, "attestation rejected: reviewer is author");
                    return Err(RoleError::NotDistinct {
                        role: self.name.clone(),
                        by: by.to_owned(),
                    });
                }
                Some(_) => {}
            }
        }
        trace!(role = %self.name, by, "attester accepted");
        Ok(())
    }
}

/// Why an obligation fails the gate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateFailure {
    /// The failing obligation.
    pub id: ObligationId,
    /// Its state (stale or open).
    pub state: ObligationState,
    /// The gating role, or `None` when a kind policy gated it.
    pub role: Option<String>,
}

/// Result of gating a run.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GateReport {
    /// Every failing obligation, in input order.
    pub failures: Vec<GateFailure>,
}

impl GateReport {
    /// Whether the run passes.
    pub fn passed(&self) -> bool {
        self.failures.is_empty()
    }
}

/// All roles plus the kind policy: the `[obligations]` table.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Policy {
    roles: BTreeMap<String, Role>,
    /// Kind-level gating (`sync`, `solver`).
    pub kinds: KindPolicy,
}

impl Policy {
    /// An empty policy: nothing gates.
    pub fn new() -> Self {
        Self::default()
    }

    /// Declare a role from its config values; selectors and `required_for` are validated.
    pub fn add_role(
        &mut self,
        name: &str,
        covers: &[&str],
        required_for: Option<&str>,
        members: Option<Vec<String>>,
        distinct_from_author: bool,
    ) -> Result<(), RoleError> {
        if self.roles.contains_key(name) {
            return Err(RoleError::Duplicate(name.to_owned()));
        }
        if covers.is_empty() {
            return Err(RoleError::EmptyCovers(name.to_owned()));
        }
        let covers = covers
            .iter()
            .map(|c| Selector::parse(c))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|source| RoleError::BadCovers {
                role: name.to_owned(),
                source,
            })?;
        let required_for = required_for
            .map(RequiredFor::parse)
            .transpose()
            .map_err(|source| RoleError::BadRequiredFor {
                role: name.to_owned(),
                source,
            })?;
        debug!(
            role = name,
            ?required_for,
            distinct_from_author,
            "role declared"
        );
        self.roles.insert(
            name.to_owned(),
            Role {
                name: name.to_owned(),
                covers,
                required_for,
                members,
                distinct_from_author,
            },
        );
        Ok(())
    }

    /// Look up a role by name.
    pub fn role(&self, name: &str) -> Option<&Role> {
        self.roles.get(name)
    }

    /// The roles whose `covers` match the obligation, in name order.
    pub fn roles_covering(&self, id: &ObligationId) -> Vec<&Role> {
        self.roles.values().filter(|r| r.covers(id)).collect()
    }

    /// Validate an attestation of `id` as `role` by `by` (coverage, membership, distinctness).
    pub fn authorize(
        &self,
        role: &str,
        id: &ObligationId,
        by: &str,
        author: Option<&str>,
    ) -> Result<(), RoleError> {
        let r = self
            .role(role)
            .ok_or_else(|| RoleError::UnknownRole(role.to_owned()))?;
        if !r.covers(id) {
            warn!(role, %id, "attestation rejected: not covered");
            return Err(RoleError::NotCovered {
                role: role.to_owned(),
                id: id.to_string(),
            });
        }
        r.check_attester(by, author)
    }

    /// Gate a run: stale or open obligations whose role or kind requires it fail.
    ///
    /// `release` is `check --release`, which also enforces the `release` ones.
    pub fn gate(
        &self,
        obligations: &[(ObligationId, ObligationState)],
        release: bool,
    ) -> GateReport {
        let mut report = GateReport::default();
        for (id, state) in obligations {
            let pending = matches!(state, ObligationState::Stale | ObligationState::Open);
            if !pending {
                continue;
            }
            let role = self
                .roles_covering(id)
                .into_iter()
                .find(|r| match r.required_for {
                    Some(RequiredFor::Check) => true,
                    Some(RequiredFor::Release) => release,
                    None => false,
                })
                .map(|r| r.name.clone());
            let by_kind = self.kinds.gates(id.kind(), *state, release);
            if role.is_some() || by_kind {
                debug!(%id, ?state, ?role, by_kind, "gate failure");
                report.failures.push(GateFailure {
                    id: id.clone(),
                    state: *state,
                    role,
                });
            }
        }
        debug!(failures = report.failures.len(), release, "gate evaluated");
        report
    }
}

/// Attempt to waive a `require`: refused unless a current attestation exists.
///
/// Attestation is sign-off, never a waiver, so even `Ok` leaves the verdict untouched
/// (obligations.md 4); a non-require is not this function's concern and is accepted.
pub fn waive_require(id: &ObligationId, attest: &AttestState) -> Result<(), RoleError> {
    if id.kind() != ObligationKind::Require {
        return Ok(());
    }
    match attest {
        AttestState::Current => Ok(()),
        _ => {
            warn!(%id, "require waiver refused: no current attestation");
            Err(RoleError::RequireWaiver(id.to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::staleness::StaleReason;

    fn id(s: &str) -> ObligationId {
        ObligationId::parse(s).unwrap()
    }

    fn policy() -> Policy {
        let mut p = Policy::new();
        p.add_role(
            "tolerance_review",
            &["fit:*", "check:*"],
            Some("release"),
            Some(vec!["ana@acme.example".into(), "bo@acme.example".into()]),
            true,
        )
        .unwrap();
        p.add_role("source", &["ots:*"], Some("check"), None, false)
            .unwrap();
        p.kinds.set("sync", "release").unwrap();
        p
    }

    // frob:tests crates/cadetta-obligations/src/roles.rs::Policy.roles_covering
    #[test]
    fn selectors_cover_the_right_nodes() {
        let p = policy();
        let names = |s: &str| -> Vec<String> {
            p.roles_covering(&id(s))
                .iter()
                .map(|r| r.name.clone())
                .collect()
        };
        assert_eq!(names("fit:Axle"), ["tolerance_review"]);
        assert_eq!(names("check:Bore"), ["tolerance_review"]);
        assert_eq!(names("ots:bearing"), ["source"]);
        assert!(names("solver:s").is_empty());
        assert!(names("require:r").is_empty());
    }

    // frob:tests crates/cadetta-obligations/src/roles.rs::Policy.gate
    #[test]
    fn check_gates_and_release_only_under_release() {
        let p = policy();
        let obs = vec![
            (id("ots:bearing"), ObligationState::Stale),
            (id("fit:Axle"), ObligationState::Open),
            (id("sync:a.sldprt"), ObligationState::Open),
            (id("check:Bore"), ObligationState::Discharged),
        ];
        let plain = p.gate(&obs, false);
        assert!(!plain.passed());
        assert_eq!(plain.failures.len(), 1);
        assert_eq!(plain.failures[0].role.as_deref(), Some("source"));
        let rel = p.gate(&obs, true);
        assert_eq!(rel.failures.len(), 3);
        assert_eq!(rel.failures[2].role, None);
        assert!(p.gate(&obs[3..], true).passed());
        assert!(Policy::new().gate(&obs, true).passed());
    }

    // frob:tests crates/cadetta-obligations/src/roles.rs::waive_require
    #[test]
    fn require_waiver_needs_attestation() {
        let r = id("require:load");
        assert_eq!(
            waive_require(&r, &AttestState::Open),
            Err(RoleError::RequireWaiver("require:load".into()))
        );
        assert!(waive_require(&r, &AttestState::Stale(StaleReason::default())).is_err());
        assert!(waive_require(&r, &AttestState::Current).is_ok());
        assert!(waive_require(&id("check:c"), &AttestState::Open).is_ok());
    }

    // frob:tests crates/cadetta-obligations/src/roles.rs::Policy.authorize
    #[test]
    fn members_and_distinct_from_author_enforced() {
        let p = policy();
        let f = id("fit:Axle");
        let ana = "ana@acme.example";
        assert!(
            p.authorize("tolerance_review", &f, ana, Some("bo@acme.example"))
                .is_ok()
        );
        assert!(matches!(
            p.authorize("tolerance_review", &f, "eve@x.org", Some("bo@acme.example")),
            Err(RoleError::NotMember { .. })
        ));
        assert!(matches!(
            p.authorize("tolerance_review", &f, ana, Some(ana)),
            Err(RoleError::NotDistinct { .. })
        ));
        assert!(matches!(
            p.authorize("tolerance_review", &f, ana, None),
            Err(RoleError::AuthorUnknown(_))
        ));
        assert!(matches!(
            p.authorize("tolerance_review", &id("ots:x"), ana, None),
            Err(RoleError::NotCovered { .. })
        ));
        assert!(matches!(
            p.authorize("nope", &f, ana, None),
            Err(RoleError::UnknownRole(_))
        ));
        // open membership and no distinctness: anyone, even the author
        assert!(
            p.authorize("source", &id("ots:x"), "anyone", Some("anyone"))
                .is_ok()
        );
    }

    // frob:tests crates/cadetta-obligations/src/roles.rs::Policy.add_role
    #[test]
    fn add_role_validates() {
        let mut p = Policy::new();
        assert!(matches!(
            p.add_role("a", &[], None, None, false),
            Err(RoleError::EmptyCovers(_))
        ));
        assert!(matches!(
            p.add_role("a", &["bogus"], None, None, false),
            Err(RoleError::BadCovers { .. })
        ));
        assert!(matches!(
            p.add_role("a", &["fit:*"], Some("never"), None, false),
            Err(RoleError::BadRequiredFor { .. })
        ));
        p.add_role("a", &["fit:*"], None, None, false).unwrap();
        assert!(matches!(
            p.add_role("a", &["fit:*"], None, None, false),
            Err(RoleError::Duplicate(_))
        ));
    }
}
