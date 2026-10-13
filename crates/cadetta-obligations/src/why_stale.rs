//! `cadetta why --stale`: the nearest changed upstream node of a stale obligation.
//!
//! An attestation stores the own digests of every transitive dep (obligations.md 3). Walking the
//! current closure breadth-first from the covered nodes, the first dep whose own digest is not in
//! the recorded set is the nearest changed node; its new value is its current own digest and the
//! old value is the recorded digest that vanished (exact when exactly one vanished, otherwise the
//! candidates are listed). When no dep changed, the covered node itself did.
//! Specification: `docs/spec/obligations.md` 2.4 and 5, `docs/spec/config-cli.md` 3.
//! frob:ticket 01M4SWN7EKZV3B1TBWNSF4YG0A

use std::collections::{BTreeSet, VecDeque};

use cadetta_ir::digest::DigestGraph;
use tracing::{debug, trace};

use crate::lock::{Attest, Hex};

/// A changed upstream node with its old and new own digests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangedNode {
    /// Graph key (IR path) of the node.
    pub node: String,
    /// Current own digest of the node.
    pub new: String,
    /// Recorded own digest it replaced, when exactly one recorded digest vanished.
    pub old: Option<String>,
    /// Hops from the covered node (0 is the covered node itself).
    pub depth: usize,
}

/// The answer to `why --stale` for one attestation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WhyStale {
    /// No attestation: the obligation is open, not stale.
    Open,
    /// The attestation is current: nothing changed.
    Current,
    /// Stale; the nearest changed node and everything else that changed.
    Stale {
        /// Nearest changed node, nearest the root.
        nearest: ChangedNode,
        /// Further changed nodes in walk order, excluding `nearest`.
        others: Vec<ChangedNode>,
        /// Recorded dep digests no longer present (candidates for the old values).
        removed: Vec<String>,
    },
}

/// Explain why `att` (covering `roots`) is stale against the current `graph`.
///
/// `current` is the current covered closure digest; returns `Err` on unknown graph keys.
pub fn why_stale(
    att: Option<&Attest>,
    roots: &[String],
    current: &str,
    graph: &DigestGraph,
    deps_of: &dyn Fn(&str) -> Vec<String>,
) -> Result<WhyStale, cadetta_ir::digest::DigestError> {
    let Some(a) = att else {
        debug!("why --stale: no attestation");
        return Ok(WhyStale::Open);
    };
    if a.digest.as_str() == current {
        debug!(id = %a.id, "why --stale: attestation current");
        return Ok(WhyStale::Current);
    }
    let recorded: BTreeSet<&str> = a.deps.iter().map(Hex::as_str).collect();
    let mut present: BTreeSet<String> = BTreeSet::new();
    let mut changed: Vec<(String, String, usize)> = Vec::new();
    let mut seen: BTreeSet<String> = roots.iter().cloned().collect();
    let mut queue: VecDeque<(String, usize)> = roots.iter().map(|r| (r.clone(), 0)).collect();
    while let Some((key, depth)) = queue.pop_front() {
        let own = graph.own(&key)?.0.clone();
        if depth > 0 {
            present.insert(own.clone());
            if !recorded.contains(own.as_str()) {
                trace!(%key, depth, "why --stale: changed dep");
                changed.push((key.clone(), own, depth));
            }
        }
        for d in deps_of(&key) {
            if seen.insert(d.clone()) {
                queue.push_back((d, depth + 1));
            }
        }
    }
    let removed: Vec<String> = recorded
        .iter()
        .filter(|h| !present.contains(**h))
        .map(|s| (*s).to_owned())
        .collect();
    let old = (removed.len() == 1).then(|| removed[0].clone());
    if changed.is_empty() {
        // The deps are unchanged, so the covered node itself changed.
        let root = roots.first().cloned().unwrap_or_default();
        let new = graph.own(&root)?.0.clone();
        debug!(%root, "why --stale: covered node itself changed");
        return Ok(WhyStale::Stale {
            nearest: ChangedNode {
                node: root,
                new,
                old: None,
                depth: 0,
            },
            others: Vec::new(),
            removed,
        });
    }
    let mut nodes = changed.into_iter().map(|(node, new, depth)| ChangedNode {
        node,
        new,
        old: old.clone(),
        depth,
    });
    let nearest = nodes.next().expect("changed is non-empty");
    debug!(node = %nearest.node, depth = nearest.depth, "why --stale: nearest changed node");
    Ok(WhyStale::Stale {
        nearest,
        others: nodes.collect(),
        removed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{ObligationId, ObligationKind, Verdict};
    use crate::lock::Date;
    use crate::staleness::{Judgement, Signer, current_state_of, record_attest};
    use cadetta_ir::model::Digest;

    fn d(s: &str) -> Digest {
        cadetta_ir::digest::digest_bytes(s.as_bytes())
    }

    fn graph(leaf: &str, mid: &str) -> DigestGraph {
        let mut g = DigestGraph::new();
        g.add_node("a", d("a"), vec!["b".into()]);
        g.add_node("b", d(mid), vec!["c".into()]);
        g.add_node("c", d(leaf), vec![]);
        g
    }

    fn deps(k: &str) -> Vec<String> {
        match k {
            "a" => vec!["b".into()],
            "b" => vec!["c".into()],
            _ => vec![],
        }
    }

    fn attest(g: &DigestGraph) -> Attest {
        let (cl, owns) = current_state_of(g, "a").unwrap();
        let signer = Signer {
            role: "lead".into(),
            by: "x@y.z".into(),
            at: Date::new("2026-10-13").unwrap(),
            reason: "r".into(),
        };
        let j = Judgement {
            verdict: Verdict::Holds,
            margin: "1".into(),
        };
        let id = ObligationId::new(ObligationKind::Check, "a");
        record_attest(id, signer, j, &cl, &owns).unwrap()
    }

    // frob:tests crates/cadetta-obligations/src/why_stale.rs::why_stale kind=unit
    #[test]
    fn nearest_changed_upstream_has_old_and_new() {
        let att = attest(&graph("c1", "b1"));
        let g = graph("c2", "b1");
        let cur = g.closure("a").unwrap();
        let r = why_stale(Some(&att), &["a".into()], &cur.0, &g, &deps).unwrap();
        let WhyStale::Stale {
            nearest, others, ..
        } = r
        else {
            panic!("not stale")
        };
        assert_eq!(nearest.node, "c");
        assert_eq!(nearest.new, d("c2").0);
        assert_eq!(nearest.old, Some(d("c1").0));
        assert!(others.is_empty());
    }

    // frob:tests crates/cadetta-obligations/src/why_stale.rs::why_stale kind=unit
    #[test]
    fn nearest_is_closest_to_root() {
        let att = attest(&graph("c1", "b1"));
        let g = graph("c2", "b2");
        let cur = g.closure("a").unwrap();
        let r = why_stale(Some(&att), &["a".into()], &cur.0, &g, &deps).unwrap();
        let WhyStale::Stale {
            nearest,
            others,
            removed,
        } = r
        else {
            panic!()
        };
        assert_eq!((nearest.node.as_str(), nearest.depth), ("b", 1));
        assert_eq!(nearest.old, None);
        assert_eq!(others[0].node, "c");
        assert_eq!(removed.len(), 2);
    }

    // frob:tests crates/cadetta-obligations/src/why_stale.rs::why_stale kind=unit
    #[test]
    fn covered_node_itself_changed() {
        let att = attest(&graph("c1", "b1"));
        let mut g = graph("c1", "b1");
        g.add_node("a", d("a2"), vec!["b".into()]);
        let cur = g.closure("a").unwrap();
        let r = why_stale(Some(&att), &["a".into()], &cur.0, &g, &deps).unwrap();
        let WhyStale::Stale { nearest, .. } = r else {
            panic!()
        };
        assert_eq!((nearest.node.as_str(), nearest.depth), ("a", 0));
    }

    // frob:tests crates/cadetta-obligations/src/why_stale.rs::why_stale kind=unit
    #[test]
    fn open_and_current() {
        let g = graph("c1", "b1");
        let att = attest(&g);
        let cur = g.closure("a").unwrap();
        assert_eq!(
            why_stale(None, &["a".into()], &cur.0, &g, &deps).unwrap(),
            WhyStale::Open
        );
        assert_eq!(
            why_stale(Some(&att), &["a".into()], &cur.0, &g, &deps).unwrap(),
            WhyStale::Current
        );
    }
}
