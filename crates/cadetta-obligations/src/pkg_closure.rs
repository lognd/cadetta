//! Dependency digests in closures and upgrade effects: what a library update makes stale.
//!
//! The package content digest is a graph node ([`crate::lock_package::add_to_graph`]) that
//! readers list among their deps, so it enters every reader's closure. Comparing the graph before
//! and after an update then gives exactly the stale obligations and the flipped verdicts,
//! whatever the version number says. Specification: `docs/spec/packages.md` 5 and 6,
//! `docs/spec/obligations.md` 3.
//! frob:ticket 01M4SXS3RXGHEG5FGTCS7X071X

use cadetta_ir::digest::{DigestError, DigestGraph};
use cadetta_ir::model::Digest;
use tracing::{debug, info};

use crate::graph::{ObligationId, ObligationState, Verdict};
use crate::lock::Attest;
use crate::lock_package::node_key;
use crate::staleness::classify_attested;

/// Add `package:NAME` to each reader's deps so the dependency digest enters its closure.
///
/// Readers are `(node key, own digest, deps)`; the package keys are appended, de-duplicated.
pub fn add_reader(
    graph: &mut DigestGraph,
    key: &str,
    own: Digest,
    mut deps: Vec<String>,
    packages: &[&str],
) {
    for p in packages {
        let k = node_key(p);
        if !deps.contains(&k) {
            deps.push(k);
        }
    }
    debug!(%key, packages = packages.len(), "reader linked to dependency digests");
    graph.add_node(key, own, deps);
}

/// A dependent obligation observed across an update.
#[derive(Debug, Clone)]
pub struct Dependent {
    /// The obligation.
    pub id: ObligationId,
    /// Graph node whose closure the obligation covers.
    pub key: String,
    /// Its recorded attestation, if any.
    pub attest: Option<Attest>,
    /// Verdict before the update (the recorded one for a recomputed kind).
    pub before: Option<Verdict>,
    /// Verdict recomputed against the updated library.
    pub after: Option<Verdict>,
    /// Whether a waiver or `allow` suppresses it.
    pub suppressed: bool,
}

/// What an update does to one dependent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Effect {
    /// The obligation.
    pub id: ObligationId,
    /// State against the updated graph.
    pub state: ObligationState,
    /// True when the closure digest changed.
    pub closure_changed: bool,
    /// `(before, after)` when the verdict flipped, whatever the version number says.
    pub flip: Option<(Verdict, Verdict)>,
}

/// Judge every dependent against the graph after an update.
///
/// `old` and `new` are the digest graphs before and after; an attested dependent is stale
/// exactly when its closure digest differs, so unrelated ones stay discharged.
pub fn upgrade_effects(
    old: &DigestGraph,
    new: &DigestGraph,
    dependents: &[Dependent],
) -> Result<Vec<Effect>, DigestError> {
    let mut out = Vec::with_capacity(dependents.len());
    for d in dependents {
        let was = old.closure(&d.key)?;
        let now = new.closure(&d.key)?;
        let deps = new.dep_owns(&d.key)?;
        let closure_changed = was != now;
        let state = classify_attested(d.attest.as_ref(), &now.0, &deps, d.suppressed);
        let flip = match (d.before, d.after) {
            (Some(b), Some(a)) if b != a => Some((b, a)),
            _ => None,
        };
        if closure_changed || flip.is_some() {
            info!(id = %d.id, ?state, closure_changed, ?flip, "update affects dependent");
        }
        out.push(Effect {
            id: d.id.clone(),
            state,
            closure_changed,
            flip,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::ObligationKind;
    use crate::lock::{Date, Hex};
    use crate::lock_package::{ResolvedPackage, add_to_graph, content_digest};

    fn world(content: &str) -> DigestGraph {
        let mut g = DigestGraph::new();
        let pkg = ResolvedPackage {
            name: "acme".into(),
            version: "1.0.0".into(),
            source: "path:../x".into(),
            digest: content_digest(&[("a", content)]),
        };
        add_to_graph(&mut g, &pkg);
        add_reader(
            &mut g,
            "app::reader",
            Digest("11".into()),
            vec![],
            &["acme"],
        );
        add_reader(&mut g, "app::other", Digest("22".into()), vec![], &[]);
        g
    }

    fn attest_for(g: &DigestGraph, key: &str, path: &str) -> Attest {
        Attest {
            id: ObligationId::new(ObligationKind::Check, path),
            role: "r".into(),
            by: "a@b".into(),
            at: Date::new("2026-10-13").unwrap(),
            digest: Hex::new(&g.closure(key).unwrap().0).unwrap(),
            verdict: Verdict::Holds,
            margin: "1 mm".into(),
            deps: g
                .dep_owns(key)
                .unwrap()
                .iter()
                .map(|d| Hex::new(&d.0).unwrap())
                .collect(),
            reason: String::new(),
        }
    }

    fn dep(g: &DigestGraph, key: &str, after: Verdict) -> Dependent {
        Dependent {
            id: ObligationId::new(ObligationKind::Check, key),
            key: key.into(),
            attest: Some(attest_for(g, key, key)),
            before: Some(Verdict::Holds),
            after: Some(after),
            suppressed: false,
        }
    }

    // frob:tests crates/cadetta-obligations/src/pkg_closure.rs::add_reader
    #[test]
    fn dependency_digest_is_in_reader_closure_only() {
        let (a, b) = (world("1"), world("2"));
        assert_ne!(
            a.closure("app::reader").unwrap(),
            b.closure("app::reader").unwrap()
        );
        assert_eq!(
            a.closure("app::other").unwrap(),
            b.closure("app::other").unwrap()
        );
    }

    // frob:tests crates/cadetta-obligations/src/pkg_closure.rs::upgrade_effects
    #[test]
    fn only_readers_go_stale() {
        let (old, new) = (world("1"), world("2"));
        let ds = [
            dep(&old, "app::reader", Verdict::Holds),
            dep(&old, "app::other", Verdict::Holds),
        ];
        let fx = upgrade_effects(&old, &new, &ds).unwrap();
        assert_eq!(fx[0].state, ObligationState::Stale);
        assert!(fx[0].closure_changed);
        assert_eq!(fx[1].state, ObligationState::Discharged);
        assert!(!fx[1].closure_changed);
    }

    // frob:tests crates/cadetta-obligations/src/pkg_closure.rs::upgrade_effects
    #[test]
    fn flip_reported_and_unchanged_library_is_quiet() {
        let (old, new) = (world("1"), world("2"));
        let ds = [dep(&old, "app::reader", Verdict::Violated)];
        let fx = upgrade_effects(&old, &new, &ds).unwrap();
        assert_eq!(fx[0].flip, Some((Verdict::Holds, Verdict::Violated)));
        let same = upgrade_effects(&old, &old, &ds).unwrap();
        assert_eq!(same[0].state, ObligationState::Discharged);
        assert!(!same[0].closure_changed);
    }

    // frob:tests crates/cadetta-obligations/src/pkg_closure.rs::upgrade_effects
    #[test]
    fn missing_node_is_an_error() {
        let g = world("1");
        let mut d = dep(&g, "app::reader", Verdict::Holds);
        d.key = "nope".into();
        assert!(upgrade_effects(&g, &g, &[d]).is_err());
    }
}
