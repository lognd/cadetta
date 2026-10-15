//! `cadetta lock verify`: well-formedness, role policy and merge-invalidated attestations.
//!
//! Specification: `docs/spec/obligations.md` section 3 and `docs/spec/config-cli.md` section 3.
//! frob:ticket 01M4SWN7A33MPRTACHTTNH3SKN

use std::collections::{BTreeMap, HashMap};
use std::fmt;

use tracing::{debug, info};

use crate::graph::ObligationId;
use crate::lock::{Lock, LockError, Record};

/// Policy of one role (obligations.md 4).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RolePolicy {
    /// Who may attest; `None` means anyone.
    pub members: Option<Vec<String>>,
    /// Whether `by` must differ from the author of the newest covered change.
    pub distinct_from_author: bool,
}

/// What verify checks the lock against.
#[derive(Debug, Clone, Default)]
pub struct VerifyContext {
    /// Live obligations and their current closure digests.
    pub digests: HashMap<ObligationId, String>,
    /// Declared roles.
    pub roles: BTreeMap<String, RolePolicy>,
    /// Author of the newest commit changing any node an obligation covers.
    pub authors: HashMap<ObligationId, String>,
}

/// One problem found in the lock.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Finding {
    /// The file is not byte-identical to its canonical rendering.
    NotCanonical,
    /// A key occurs on more than one line.
    DuplicateKey {
        /// Number of lines lost to the tie rule.
        extra: usize,
    },
    /// An attestation names an obligation that does not exist.
    UnknownObligation(ObligationId),
    /// An attestation names an undeclared role.
    UnknownRole {
        /// The obligation.
        id: ObligationId,
        /// The role.
        role: String,
    },
    /// The attester is not in the role's `members`.
    NotMember {
        /// The obligation.
        id: ObligationId,
        /// The role.
        role: String,
        /// The attester.
        by: String,
    },
    /// The attester is the author but the role requires someone else.
    NotDistinct {
        /// The obligation.
        id: ObligationId,
        /// The role.
        role: String,
        /// The attester.
        by: String,
    },
    /// The recorded digest no longer matches the current closure digest (e.g. after a merge).
    Invalidated {
        /// The obligation.
        id: ObligationId,
        /// The role.
        role: String,
        /// Digest recorded in the lock.
        recorded: String,
        /// Current closure digest.
        current: String,
    },
}

// frob:ticket 01M4X3W8DK8NBX5A3GS0DM3P8Y
/// Schema tag of the JSON report; the schema is `docs/schemas/lock.json`.
pub const REPORT_SCHEMA: &str = "cadetta.lock/1";

impl Finding {
    /// Stable machine-readable kind of the finding (the `kind` field of the JSON report).
    pub fn kind(&self) -> &'static str {
        match self {
            Self::NotCanonical => "not_canonical",
            Self::DuplicateKey { .. } => "duplicate_key",
            Self::UnknownObligation(_) => "unknown_obligation",
            Self::UnknownRole { .. } => "unknown_role",
            Self::NotMember { .. } => "not_member",
            Self::NotDistinct { .. } => "not_distinct",
            Self::Invalidated { .. } => "invalidated",
        }
    }

    // frob:ticket 01M4X3W8DK8NBX5A3GS0DM3P8Y
    /// The finding as a JSON object: `kind`, `message` and the fields its variant carries.
    pub fn to_json(&self) -> serde_json::Value {
        let mut v = serde_json::json!({"kind": self.kind(), "message": self.to_string()});
        let obj = v.as_object_mut().expect("json! object literal");
        let mut put = |k: &str, val: serde_json::Value| {
            obj.insert(k.to_owned(), val);
        };
        match self {
            Self::NotCanonical => {}
            Self::DuplicateKey { extra } => put("extra", (*extra).into()),
            Self::UnknownObligation(id) => put("id", id.to_string().into()),
            Self::UnknownRole { id, role } => {
                put("id", id.to_string().into());
                put("role", role.as_str().into());
            }
            Self::NotMember { id, role, by } | Self::NotDistinct { id, role, by } => {
                put("id", id.to_string().into());
                put("role", role.as_str().into());
                put("by", by.as_str().into());
            }
            Self::Invalidated {
                id,
                role,
                recorded,
                current,
            } => {
                put("id", id.to_string().into());
                put("role", role.as_str().into());
                put("recorded", recorded.as_str().into());
                put("current", current.as_str().into());
            }
        }
        v
    }
}

impl fmt::Display for Finding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotCanonical => write!(f, "lock is not in canonical sorted form"),
            Self::DuplicateKey { extra } => write!(f, "{extra} duplicate key line(s)"),
            Self::UnknownObligation(id) => write!(f, "attest {id}: no such obligation"),
            Self::UnknownRole { id, role } => write!(f, "attest {id}: unknown role {role}"),
            Self::NotMember { id, role, by } => {
                write!(f, "attest {id}: {by} is not a member of {role}")
            }
            Self::NotDistinct { id, role, by } => {
                write!(
                    f,
                    "attest {id}: {by} authored the change; {role} needs another"
                )
            }
            Self::Invalidated {
                id,
                role,
                recorded,
                current,
            } => write!(
                f,
                "attest {id} role={role}: invalidated, recorded digest {recorded} != current {current}"
            ),
        }
    }
}

/// The result of a verify pass.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VerifyReport {
    /// Everything found, in lock order.
    pub findings: Vec<Finding>,
}

impl VerifyReport {
    /// Whether nothing was found.
    pub fn is_clean(&self) -> bool {
        self.findings.is_empty()
    }

    /// Process exit status: 1 on any finding, else 0.
    pub fn exit_code(&self) -> i32 {
        i32::from(!self.is_clean())
    }

    // frob:ticket 01M4X3W8DK8NBX5A3GS0DM3P8Y
    /// The `cadetta lock verify --format json` report (`docs/schemas/lock.json`).
    pub fn to_json(&self) -> serde_json::Value {
        debug!(
            findings = self.findings.len(),
            "rendering lock verify report"
        );
        serde_json::json!({
            "schema": REPORT_SCHEMA,
            "clean": self.is_clean(),
            "exit_code": self.exit_code(),
            "findings": self.findings.iter().map(Finding::to_json).collect::<Vec<_>>(),
        })
    }

    /// The attestations invalidated by a merge or later change.
    pub fn invalidated(&self) -> impl Iterator<Item = &Finding> {
        self.findings
            .iter()
            .filter(|f| matches!(f, Finding::Invalidated { .. }))
    }
}

/// Verify lock `text` against `ctx`.
pub fn verify_text(text: &str, ctx: &VerifyContext) -> Result<VerifyReport, LockError> {
    let lock = Lock::parse(text)?;
    let mut findings = Vec::new();
    let lines = text.lines().filter(|l| !l.is_empty()).count();
    if lines > lock.len() {
        findings.push(Finding::DuplicateKey {
            extra: lines - lock.len(),
        });
    }
    if lock.render() != text {
        findings.push(Finding::NotCanonical);
    }
    for rec in lock.records() {
        let Record::Attest(a) = rec else { continue };
        let Some(current) = ctx.digests.get(&a.id) else {
            findings.push(Finding::UnknownObligation(a.id.clone()));
            continue;
        };
        match ctx.roles.get(&a.role) {
            None => findings.push(Finding::UnknownRole {
                id: a.id.clone(),
                role: a.role.clone(),
            }),
            Some(p) => {
                if p.members.as_ref().is_some_and(|m| !m.contains(&a.by)) {
                    findings.push(Finding::NotMember {
                        id: a.id.clone(),
                        role: a.role.clone(),
                        by: a.by.clone(),
                    });
                }
                if p.distinct_from_author && ctx.authors.get(&a.id) == Some(&a.by) {
                    findings.push(Finding::NotDistinct {
                        id: a.id.clone(),
                        role: a.role.clone(),
                        by: a.by.clone(),
                    });
                }
            }
        }
        if a.digest.as_str() != current {
            debug!(id = %a.id, "attestation digest differs from current");
            findings.push(Finding::Invalidated {
                id: a.id.clone(),
                role: a.role.clone(),
                recorded: a.digest.as_str().to_owned(),
                current: current.clone(),
            });
        }
    }
    info!(findings = findings.len(), "lock verified");
    Ok(VerifyReport { findings })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn oid(s: &str) -> ObligationId {
        ObligationId::parse(s).unwrap()
    }

    fn line(id: &str, digest: &str, by: &str) -> String {
        format!(
            "attest {id} role=r by={by} at=2026-01-01 digest={digest} verdict=holds margin=1mm deps= reason=\"ok\"\n"
        )
    }

    fn ctx(current: &str) -> VerifyContext {
        let mut c = VerifyContext::default();
        c.digests.insert(oid("check:a"), current.to_owned());
        c.roles.insert("r".into(), RolePolicy::default());
        c
    }

    // frob:tests crates/cadetta-obligations/src/lock_verify.rs::verify_text
    #[test]
    fn merged_lock_lists_invalidated_and_exits_1() {
        let rep = verify_text(&line("check:a", "aaaa", "x@y"), &ctx("bbbb")).unwrap();
        assert_eq!(rep.invalidated().count(), 1);
        assert_eq!(rep.exit_code(), 1);
    }

    // frob:tests crates/cadetta-obligations/src/lock_verify.rs::verify_text
    #[test]
    fn current_lock_is_clean() {
        let rep = verify_text(&line("check:a", "aaaa", "x@y"), &ctx("aaaa")).unwrap();
        assert!(rep.is_clean());
        assert_eq!(rep.exit_code(), 0);
    }

    // frob:tests crates/cadetta-obligations/src/lock_verify.rs::verify_text
    #[test]
    fn structural_and_policy_findings() {
        let mut c = ctx("aaaa");
        c.roles.insert(
            "r".into(),
            RolePolicy {
                members: Some(vec!["ok@y".into()]),
                distinct_from_author: true,
            },
        );
        c.authors.insert(oid("check:a"), "me@y".into());
        let text = format!(
            "{}{}{}",
            line("check:z", "aaaa", "ok@y"),
            line("check:a", "aaaa", "me@y"),
            line("check:a", "aaaa", "me@y")
        );
        let rep = verify_text(&text, &c).unwrap();
        assert!(rep.findings.contains(&Finding::NotCanonical));
        assert!(rep.findings.contains(&Finding::DuplicateKey { extra: 1 }));
        assert!(
            rep.findings
                .contains(&Finding::UnknownObligation(oid("check:z")))
        );
        assert!(
            rep.findings
                .iter()
                .any(|f| matches!(f, Finding::NotMember { .. }))
        );
        assert!(
            rep.findings
                .iter()
                .any(|f| matches!(f, Finding::NotDistinct { .. }))
        );
    }

    // frob:tests crates/cadetta-obligations/src/lock_verify.rs::VerifyReport.to_json
    // frob:tests crates/cadetta-obligations/src/lock_verify.rs::Finding.to_json
    #[test]
    fn json_report_validates_against_lock_schema() {
        let schema: serde_json::Value =
            serde_json::from_str(include_str!("../../../docs/schemas/lock.json")).unwrap();
        let validator = jsonschema::validator_for(&schema).unwrap();
        let mut c = ctx("aaaa");
        c.roles.insert(
            "r".into(),
            RolePolicy {
                members: Some(vec!["ok@y".into()]),
                distinct_from_author: true,
            },
        );
        c.authors.insert(oid("check:a"), "me@y".into());
        let text = format!(
            "{}{}{}",
            line("check:z", "aaaa", "ok@y"),
            line("check:a", "bbbb", "me@y"),
            line("check:a", "bbbb", "me@y")
        );
        let rep = verify_text(&text, &c).unwrap();
        let v = rep.to_json();
        assert!(validator.is_valid(&v), "{v}");
        assert_eq!(v["clean"], false);
        assert_eq!(v["exit_code"], 1);
        assert!(v["findings"].as_array().unwrap().len() >= 5);
        let clean = verify_text(&line("check:a", "aaaa", "x@y"), &ctx("aaaa"))
            .unwrap()
            .to_json();
        assert!(validator.is_valid(&clean), "{clean}");
        assert_eq!(clean["clean"], true);
        assert_eq!(clean["findings"].as_array().unwrap().len(), 0);
    }

    // frob:tests crates/cadetta-obligations/src/lock_verify.rs::verify_text
    #[test]
    fn unknown_role_is_flagged() {
        let mut c = ctx("aaaa");
        c.roles.clear();
        let rep = verify_text(&line("check:a", "aaaa", "x@y"), &c).unwrap();
        assert!(matches!(rep.findings[0], Finding::UnknownRole { .. }));
    }
}
