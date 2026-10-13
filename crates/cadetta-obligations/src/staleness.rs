//! Attestations and staleness, derived from recorded versus current closure digests.
//!
//! Staleness is computed, never stored: an attestation records the closure digest of the nodes
//! it covers and the own digests of their transitive deps; comparing those with the current
//! graph gives the state and, for `why --stale`, which deps changed. Specification:
//! `docs/spec/obligations.md` 2.4 and 4, `docs/spec/ir.md` 4.
//! frob:ticket 01M4ST8ERHSJXGDXNH75ZCX6H7

use std::collections::BTreeSet;

use cadetta_ir::digest::{DigestError, DigestGraph, digest_bytes};
use cadetta_ir::model::Digest;
use tracing::{debug, trace};

use crate::graph::{ObligationId, ObligationState, Verdict};
use crate::lock::{Attest, Date, Hex};

/// Why an attestation is no longer current: which dep own digests appeared or disappeared.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StaleReason {
    /// Dep own digests present now but absent when attested (changed or new deps).
    pub added: Vec<String>,
    /// Dep own digests recorded at attestation but gone now (changed or dropped deps).
    pub removed: Vec<String>,
}

impl StaleReason {
    /// Whether the covered set itself changed while the deps' own digests did not.
    pub fn only_covered_changed(&self) -> bool {
        self.added.is_empty() && self.removed.is_empty()
    }
}

/// State of one attestation against the current graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttestState {
    /// No attestation exists.
    Open,
    /// The recorded closure digest equals the current one.
    Current,
    /// Something covered changed; never auto-discharged, even when the verdict is unchanged.
    Stale(StaleReason),
}

/// Compare an attestation with the current closure digest and dep own digests.
pub fn attest_state(att: Option<&Attest>, current: &str, current_deps: &[Digest]) -> AttestState {
    let Some(a) = att else {
        trace!("no attestation: open");
        return AttestState::Open;
    };
    if a.digest.as_str() == current {
        return AttestState::Current;
    }
    let old: BTreeSet<&str> = a.deps.iter().map(Hex::as_str).collect();
    let new: BTreeSet<&str> = current_deps.iter().map(|d| d.0.as_str()).collect();
    let reason = StaleReason {
        added: new.difference(&old).map(|s| (*s).to_owned()).collect(),
        removed: old.difference(&new).map(|s| (*s).to_owned()).collect(),
    };
    debug!(id = %a.id, added = reason.added.len(), removed = reason.removed.len(), "attestation stale");
    AttestState::Stale(reason)
}

/// Closure digest of the set an obligation covers: one closure as is, several combined.
///
/// Several instance closures (`wheel[0..3]` with differing arguments) are sorted, de-duplicated
/// and hashed together, so any member changing changes the set digest.
pub fn covered_digest(closures: &[Digest]) -> Digest {
    let set: BTreeSet<&str> = closures.iter().map(|d| d.0.as_str()).collect();
    if set.len() == 1 {
        return Digest(set.into_iter().next().unwrap_or_default().to_owned());
    }
    let joined = set.into_iter().collect::<Vec<_>>().join("\n");
    digest_bytes(joined.as_bytes())
}

/// The verdict and margin an attestation signs off on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Judgement {
    /// Verdict at ack time.
    pub verdict: Verdict,
    /// Margin at ack time (quantity text).
    pub margin: String,
}

/// Who attests, in which role, and why (the non-digest fields of an `attest` record).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signer {
    /// Role attesting.
    pub role: String,
    /// Identity (`git config user.email` or `--as`).
    pub by: String,
    /// Date of the ack.
    pub at: Date,
    /// Free text.
    pub reason: String,
}

/// Build the `attest` record for `id`, recording `judgement` and the deps' own digests.
pub fn record_attest(
    id: ObligationId,
    signer: Signer,
    judgement: Judgement,
    covered: &Digest,
    dep_owns: &[Digest],
) -> Result<Attest, String> {
    let mut deps = dep_owns
        .iter()
        .map(|d| Hex::new(&d.0))
        .collect::<Result<Vec<_>, _>>()?;
    deps.sort();
    deps.dedup();
    debug!(%id, role = %signer.role, deps = deps.len(), "attestation recorded");
    Ok(Attest {
        id,
        role: signer.role,
        by: signer.by,
        at: signer.at,
        digest: Hex::new(&covered.0)?,
        verdict: judgement.verdict,
        margin: judgement.margin,
        deps,
        reason: signer.reason,
    })
}

/// Current closure and dep own digests of a node, for judging attestations on it.
pub fn current_state_of(
    graph: &DigestGraph,
    key: &str,
) -> Result<(Digest, Vec<Digest>), DigestError> {
    Ok((graph.closure(key)?, graph.dep_owns(key)?))
}

/// Classify a sign-off obligation (any kind) from its attestation, derived on every call.
pub fn classify_attested(
    att: Option<&Attest>,
    current: &str,
    current_deps: &[Digest],
    suppressed: bool,
) -> ObligationState {
    let out = if suppressed {
        ObligationState::Waived
    } else {
        match attest_state(att, current, current_deps) {
            AttestState::Open => ObligationState::Open,
            AttestState::Current => ObligationState::Discharged,
            AttestState::Stale(_) => ObligationState::Stale,
        }
    };
    debug!(?out, "attested obligation classified");
    out
}

/// Classify a `require` (obligations.md 4): sign-off never changes a verdict.
///
/// A `holds` require is discharged without sign-off when `signoff_configured` is false, and
/// only with a current attestation when it is true (open or stale otherwise, O003). A violated
/// or undecided require keeps its verdict whatever the attestation says.
pub fn classify_require(
    verdict: Option<Verdict>,
    signoff_configured: bool,
    state: &AttestState,
) -> ObligationState {
    let out = match verdict {
        Some(Verdict::Holds) if signoff_configured => match state {
            AttestState::Current => ObligationState::Discharged,
            AttestState::Open => ObligationState::Open,
            AttestState::Stale(_) => ObligationState::Stale,
        },
        Some(Verdict::Holds) => ObligationState::Discharged,
        Some(Verdict::Violated) => ObligationState::Violated,
        Some(Verdict::Undecided) | None => ObligationState::Undecided,
    };
    debug!(?verdict, signoff_configured, ?out, "require classified");
    out
}

/// Classify an as-built obligation (OQ-13): an inspection recorded as an attestation.
///
/// It is stale if the design changed after the inspection, violated if the inspection itself
/// found the part out of tolerance and the design is unchanged, and open without inspection.
pub fn classify_as_built(inspection: Option<&Attest>, current: &str) -> ObligationState {
    let out = match inspection {
        None => ObligationState::Open,
        Some(a) if a.digest.as_str() != current => ObligationState::Stale,
        Some(a) => match a.verdict {
            Verdict::Holds => ObligationState::Discharged,
            Verdict::Violated => ObligationState::Violated,
            Verdict::Undecided => ObligationState::Undecided,
        },
    };
    debug!(?out, "as-built classified");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadetta_ir::model::{Member, MemberKind, Part};
    use cadetta_ir::path::{Path, PathContext};

    fn p(s: &str) -> Path {
        Path::parse(s, PathContext::General).unwrap()
    }

    fn mem(path: &str, deps: &[&str], default: &str) -> Member {
        let mut m = Member::new(p(path), MemberKind::Param);
        m.deps = deps.iter().map(|d| p(d)).collect();
        m.attrs.insert("default".into(), default.into());
        m
    }

    fn graph(ms: &[Member]) -> DigestGraph {
        let mut g = DigestGraph::new();
        for m in ms {
            g.add_member(m);
        }
        g
    }

    fn chain(far: &str) -> Vec<Member> {
        vec![
            mem("a.b::X.far", &[], far),
            mem("a.b::X.mid", &["a.b::X.far"], "m"),
            mem("a.b::X.chk", &["a.b::X.mid"], "c"),
            mem("a.b::X.other", &[], "o"),
        ]
    }

    fn attest(g: &DigestGraph, key: &str, v: Verdict) -> Attest {
        let (c, deps) = current_state_of(g, key).unwrap();
        record_attest(
            ObligationId::parse(&format!("check:{key}")).unwrap(),
            Signer {
                role: "r".into(),
                by: "a@x.example".into(),
                at: Date::new("2026-01-02").unwrap(),
                reason: "ok".into(),
            },
            Judgement {
                verdict: v,
                margin: "0.1 mm".into(),
            },
            &c,
            &deps,
        )
        .unwrap()
    }

    fn state(g: &DigestGraph, key: &str, a: &Attest) -> AttestState {
        let (c, deps) = current_state_of(g, key).unwrap();
        attest_state(Some(a), &c.0, &deps)
    }

    // frob:tests crates/cadetta-obligations/src/staleness.rs::attest_state
    #[test]
    fn upstream_change_any_depth_stales() {
        let g0 = graph(&chain("f"));
        let a = attest(&g0, "a.b::X.chk", Verdict::Holds);
        assert_eq!(state(&g0, "a.b::X.chk", &a), AttestState::Current);
        let g1 = graph(&chain("changed"));
        let AttestState::Stale(r) = state(&g1, "a.b::X.chk", &a) else {
            panic!("expected stale");
        };
        assert_eq!(r.added.len(), 1);
        assert_eq!(r.removed.len(), 1);
    }

    // frob:tests crates/cadetta-obligations/src/staleness.rs::attest_state
    #[test]
    fn unrelated_change_stays_discharged() {
        let g0 = graph(&chain("f"));
        let a = attest(&g0, "a.b::X.chk", Verdict::Holds);
        let mut ms = chain("f");
        ms[3].attrs.insert("default".into(), "different".into());
        assert_eq!(state(&graph(&ms), "a.b::X.chk", &a), AttestState::Current);
    }

    // frob:tests crates/cadetta-obligations/src/staleness.rs::attest_state
    #[test]
    fn staleness_is_derived_not_stored() {
        // The lock render carries no state field; the same record yields different states
        // purely from the graph passed in.
        let g0 = graph(&chain("f"));
        let a = attest(&g0, "a.b::X.chk", Verdict::Holds);
        let line = crate::lock::Record::Attest(a.clone()).render();
        assert!(!line.contains("stale") && !line.contains("state"));
        assert_eq!(state(&g0, "a.b::X.chk", &a), AttestState::Current);
        let g1 = graph(&chain("x"));
        assert!(matches!(
            state(&g1, "a.b::X.chk", &a),
            AttestState::Stale(_)
        ));
        assert_eq!(attest_state(None, "aa", &[]), AttestState::Open);
    }

    fn part(path: &str, primary: &str) -> Part {
        Part {
            path: p(path),
            type_name: "Iface".into(),
            count: None,
            primary: primary.into(),
            alternates: Vec::new(),
            args: [("d".to_owned(), "1".to_owned())].into(),
        }
    }

    fn part_graph(primary: &str) -> DigestGraph {
        let mut g = graph(&[mem("a.b::R.seat", &[], "s")]);
        g.add_part("a.b::R.axle", &part("a.b::R.axle", primary));
        g.add_part("a.b::R.frame", &part("a.b::R.frame", "F"));
        g
    }

    // frob:tests crates/cadetta-obligations/src/staleness.rs::covered_digest
    #[test]
    fn interface_slot_swap_stales_only_that_slot() {
        let g0 = part_graph("A");
        let g1 = part_graph("B");
        let inst = |g: &DigestGraph, slot: &str, i: &str| {
            g.instance_closure("a.b::R.seat", i, Some(slot)).unwrap()
        };
        let axle0 = covered_digest(&[inst(&g0, "a.b::R.axle", "R/axle[0].seat")]);
        let frame0 = covered_digest(&[inst(&g0, "a.b::R.frame", "R/frame[0].seat")]);
        let axle1 = covered_digest(&[inst(&g1, "a.b::R.axle", "R/axle[0].seat")]);
        let frame1 = covered_digest(&[inst(&g1, "a.b::R.frame", "R/frame[0].seat")]);
        let st = |rec: &Digest, now: &Digest| {
            classify_attested(Some(&attest_with(rec)), &now.0, &[], false)
        };
        assert_eq!(st(&axle0, &axle1), ObligationState::Stale);
        assert_eq!(st(&frame0, &frame1), ObligationState::Discharged);
    }

    fn attest_with(c: &Digest) -> Attest {
        record_attest(
            ObligationId::parse("check:a::c").unwrap(),
            Signer {
                role: "r".into(),
                by: "a@x.example".into(),
                at: Date::new("2026-01-02").unwrap(),
                reason: "ok".into(),
            },
            Judgement {
                verdict: Verdict::Holds,
                margin: "1".into(),
            },
            c,
            &[],
        )
        .unwrap()
    }

    // frob:tests crates/cadetta-obligations/src/staleness.rs::covered_digest
    #[test]
    fn covered_set_digest_tracks_every_member() {
        let (a, b, c) = (
            Digest("aa".into()),
            Digest("bb".into()),
            Digest("cc".into()),
        );
        assert_eq!(covered_digest(std::slice::from_ref(&a)), a);
        assert_eq!(covered_digest(&[a.clone(), a.clone()]), a);
        assert_eq!(
            covered_digest(&[a.clone(), b.clone()]),
            covered_digest(&[b.clone(), a.clone()])
        );
        assert_ne!(covered_digest(&[a.clone(), b]), covered_digest(&[a, c]));
    }

    // frob:tests crates/cadetta-obligations/src/staleness.rs::classify_require
    #[test]
    fn require_discharged_only_with_current_attestation() {
        use ObligationState as S;
        let stale = AttestState::Stale(StaleReason::default());
        let h = Some(Verdict::Holds);
        assert_eq!(
            classify_require(h, true, &AttestState::Current),
            S::Discharged
        );
        assert_eq!(classify_require(h, true, &AttestState::Open), S::Open);
        assert_eq!(classify_require(h, true, &stale), S::Stale);
        assert_eq!(
            classify_require(h, false, &AttestState::Open),
            S::Discharged
        );
        // sign-off never changes a violated verdict
        assert_eq!(
            classify_require(Some(Verdict::Violated), true, &AttestState::Current),
            S::Violated
        );
        assert_eq!(
            classify_require(None, true, &AttestState::Current),
            S::Undecided
        );
    }

    // frob:tests crates/cadetta-obligations/src/staleness.rs::classify_as_built
    #[test]
    fn as_built_stale_when_design_changed_after_inspection() {
        let a = attest_with(&Digest("aa".into()));
        assert_eq!(
            classify_as_built(Some(&a), "aa"),
            ObligationState::Discharged
        );
        assert_eq!(classify_as_built(Some(&a), "bb"), ObligationState::Stale);
        assert_eq!(classify_as_built(None, "aa"), ObligationState::Open);
        let mut bad = a;
        bad.verdict = Verdict::Violated;
        assert_eq!(
            classify_as_built(Some(&bad), "aa"),
            ObligationState::Violated
        );
        assert_eq!(classify_as_built(Some(&bad), "bb"), ObligationState::Stale);
    }

    // frob:tests crates/cadetta-obligations/src/staleness.rs::record_attest
    #[test]
    fn attestation_records_verdict_margin_and_dep_owns() {
        let g = graph(&chain("f"));
        let a = attest(&g, "a.b::X.chk", Verdict::Holds);
        assert_eq!(a.verdict, Verdict::Holds);
        assert_eq!(a.margin, "0.1 mm");
        let owns = g.dep_owns("a.b::X.chk").unwrap();
        assert_eq!(owns.len(), 2);
        assert_eq!(
            a.deps.iter().map(|h| h.as_str()).collect::<Vec<_>>(),
            owns.iter().map(|d| d.0.as_str()).collect::<Vec<_>>()
        );
        assert!(a.deps.windows(2).all(|w| w[0] < w[1]));
        assert!(
            record_attest(
                a.id.clone(),
                Signer {
                    role: "r".into(),
                    by: "b".into(),
                    at: a.at.clone(),
                    reason: String::new()
                },
                Judgement {
                    verdict: Verdict::Holds,
                    margin: String::new()
                },
                &Digest("ZZ".into()),
                &[],
            )
            .is_err()
        );
    }

    // frob:tests crates/cadetta-obligations/src/staleness.rs::classify_attested
    #[test]
    fn attested_kinds_classify_via_closure() {
        let a = attest_with(&Digest("aa".into()));
        assert_eq!(
            classify_attested(Some(&a), "aa", &[], false),
            ObligationState::Discharged
        );
        assert_eq!(
            classify_attested(Some(&a), "bb", &[], false),
            ObligationState::Stale
        );
        assert_eq!(
            classify_attested(None, "bb", &[], false),
            ObligationState::Open
        );
    }
}
