//! The `waiver` obligation kind and the lock `waiver` record.
//!
//! A `waive` declaration becomes a `waiver:<glob>` obligation whose `waives` edges reach every
//! matching target node; `ack waiver:GLOB` stores the closure digest of those targets in the
//! lock, and a later differing digest is W002 (stale). Specification: `docs/spec/obligations.md`
//! 2 and 3, `docs/spec/lints.md` 2.1.
//! frob:ticket 01M4SWN76NXYQCYDG7C1A8C2KG

use cadetta_ir::model::Digest;
use tracing::{debug, info};

use crate::graph::{EdgeKind, Graph, GraphError, NodeId, NodeKind, ObligationId, ObligationKind};
use crate::lock::{AltBinding, Hex, Lock, Record, Waiver};
use crate::selector::{Glob, SelectorError};
use crate::staleness::covered_digest;

/// The lint code a stale waiver raises.
pub const STALE_CODE: &str = "W002";

/// A `waive` declaration: the lock key of its record (code, glob, alt).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WaiverDecl {
    /// Diagnostic code waived.
    pub code: String,
    /// Target path glob.
    pub glob: String,
    /// Optional alternate restriction.
    pub alt: Option<AltBinding>,
}

impl WaiverDecl {
    /// The obligation id of the declaration, `waiver:<glob>`.
    pub fn obligation_id(&self) -> ObligationId {
        ObligationId::new(ObligationKind::Waiver, self.glob.clone())
    }
}

/// Review state of a waiver against the lock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaiverState {
    /// No lock record yet: suppresses, review is an open `waiver` obligation (O002).
    Open,
    /// The recorded digest equals the current target digest.
    Current,
    /// A target changed since review: W002, the suppressed diagnostic reappears.
    Stale,
}

impl WaiverState {
    /// The lint code the state raises, if any (`W002` for [`WaiverState::Stale`]).
    pub fn code(self) -> Option<&'static str> {
        (self == WaiverState::Stale).then_some(STALE_CODE)
    }
}

/// Add the waiver node, its `waives` edges to every target matching the glob, and return the
/// obligation id with the node.
///
/// `candidates` are the graph nodes a waiver may target; the node digest is the closure digest.
pub fn graph_waiver(
    g: &mut Graph,
    decl: &WaiverDecl,
    candidates: &[NodeId],
) -> Result<(ObligationId, NodeId), WaiverError> {
    let glob = Glob::new(&decl.glob).map_err(WaiverError::Glob)?;
    let w = g.add_node(NodeKind::Waiver, &decl.glob, None);
    let mut n = 0;
    for &c in candidates {
        let Some(node) = g.node(c) else { continue };
        if glob.matches(&node.path) {
            g.add_edge(w, EdgeKind::Waives, c)
                .map_err(WaiverError::Graph)?;
            n += 1;
        }
    }
    debug!(code = %decl.code, glob = %decl.glob, targets = n, "waiver graphed");
    Ok((decl.obligation_id(), w))
}

/// Closure digest of a waiver's targets: the combined digest of every `waives` target.
///
/// Targets without a digest are skipped.
pub fn target_digest(g: &Graph, w: NodeId) -> Option<Digest> {
    let ds: Vec<Digest> = g
        .successors(w, EdgeKind::Waives)
        .into_iter()
        .filter_map(|t| g.node(t)?.digest.clone().map(Digest))
        .collect();
    (!ds.is_empty()).then(|| covered_digest(&ds))
}

/// Compare the lock record of `decl` with the current target `digest`.
pub fn waiver_state(lock: &Lock, decl: &WaiverDecl, digest: &Digest) -> WaiverState {
    match lock.get(&record_of(decl, Hex::new("0").expect("valid hex")).key()) {
        Some(Record::Waiver(w)) if w.digest.as_str() == digest.0 => WaiverState::Current,
        Some(_) => {
            info!(code = %decl.code, glob = %decl.glob, "waiver stale (W002)");
            WaiverState::Stale
        }
        None => WaiverState::Open,
    }
}

/// Record (or re-review) the waiver: replace any record with this key by one binding `digest`.
pub fn ack_waiver(lock: &mut Lock, decl: &WaiverDecl, digest: &Digest) -> Result<(), WaiverError> {
    let hex = Hex::new(&digest.0).map_err(WaiverError::Digest)?;
    let rec = record_of(decl, hex);
    lock.remove(&rec.key());
    lock.insert(rec);
    info!(code = %decl.code, glob = %decl.glob, "waiver reviewed");
    Ok(())
}

fn record_of(decl: &WaiverDecl, digest: Hex) -> Record {
    Record::Waiver(Waiver {
        code: decl.code.clone(),
        glob: decl.glob.clone(),
        alt: decl.alt.clone(),
        digest,
    })
}

/// Errors from graphing or acknowledging a waiver.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum WaiverError {
    /// The waiver glob is malformed.
    #[error("bad waiver glob: {0}")]
    Glob(SelectorError),
    /// The graph rejected a `waives` edge.
    #[error("graph error: {0}")]
    Graph(GraphError),
    /// The digest is not lowercase hex.
    #[error("bad digest: {0}")]
    Digest(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decl() -> WaiverDecl {
        WaiverDecl {
            code: "C001".into(),
            glob: "acme::Axle.*".into(),
            alt: None,
        }
    }

    fn graph(d1: &str) -> (Graph, NodeId, Vec<NodeId>) {
        let mut g = Graph::new();
        let a = g.add_node(NodeKind::Check, "acme::Axle.seat", Some(d1));
        let b = g.add_node(NodeKind::Check, "acme::Axle.bore", Some("bb"));
        let c = g.add_node(NodeKind::Check, "acme::Other.x", Some("cc"));
        let (_, w) = graph_waiver(&mut g, &decl(), &[a, b, c]).unwrap();
        (g, w, vec![a, b, c])
    }

    // frob:tests crates/cadetta-obligations/src/waiver.rs::graph_waiver
    #[test]
    fn waiver_obligation_and_edges_exist() {
        let (g, w, n) = graph("aa");
        assert_eq!(decl().obligation_id().to_string(), "waiver:acme::Axle.*");
        assert_eq!(g.successors(w, EdgeKind::Waives), vec![n[0], n[1]]);
    }

    // frob:tests crates/cadetta-obligations/src/waiver.rs::ack_waiver
    #[test]
    fn lock_record_binds_digest_and_changed_digest_is_w002() {
        let (g, w, _) = graph("aa");
        let d = target_digest(&g, w).unwrap();
        let mut lock = Lock::new();
        assert_eq!(waiver_state(&lock, &decl(), &d), WaiverState::Open);
        ack_waiver(&mut lock, &decl(), &d).unwrap();
        assert!(lock.render().starts_with("waiver C001 "));
        assert_eq!(waiver_state(&lock, &decl(), &d), WaiverState::Current);

        let (g2, w2, _) = graph("a1");
        let d2 = target_digest(&g2, w2).unwrap();
        let s = waiver_state(&lock, &decl(), &d2);
        assert_eq!(s, WaiverState::Stale);
        assert_eq!(s.code(), Some("W002"));

        ack_waiver(&mut lock, &decl(), &d2).unwrap();
        assert_eq!(waiver_state(&lock, &decl(), &d2), WaiverState::Current);
    }
}
