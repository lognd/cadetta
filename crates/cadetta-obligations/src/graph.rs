//! Obligation graph nodes, edges, obligation ids and states.
//!
//! Specification: `docs/spec/obligations.md` sections 1 and 2.
//! frob:ticket 01M4ST8EMQTXAGZAWGE2M5A4BR

use std::fmt;

use tracing::{debug, trace, warn};

/// Kind of an IR entity that appears as a node of the graph (obligations.md 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NodeKind {
    /// A parameter.
    Param,
    /// A bound on a parameter.
    Bound,
    /// A citation.
    Citation,
    /// A data table.
    Table,
    /// A fit.
    Fit,
    /// A check.
    Check,
    /// A require.
    Require,
    /// A solver.
    Solver,
    /// A solver result file.
    SolverResult,
    /// A component (including `ots` components).
    Component,
    /// An interface.
    Interface,
    /// A part, with its alternates.
    Part,
    /// A binding of a parameter to a native locator.
    Binding,
    /// A native CAD file.
    NativeFile,
    /// An attestation.
    Attestation,
    /// A waiver.
    Waiver,
}

/// Edge kind of the graph, exactly the ten of the obligations.md 1 table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum EdgeKind {
    /// param -> each param its expression reads.
    Derives,
    /// param, table, component -> citation.
    Cites,
    /// component -> interface.
    Implements,
    /// part -> primary and each alternate.
    Instantiates,
    /// check, require, fit -> what it reads.
    Evaluates,
    /// solver output -> solver inputs, script, deps.
    Solves,
    /// param -> native locator.
    Binds,
    /// native file -> bound params.
    Realizes,
    /// attestation -> a set of node digests.
    Attests,
    /// waiver -> target.
    Waives,
}

impl EdgeKind {
    /// All edge kinds, in table order.
    pub const ALL: [EdgeKind; 10] = [
        EdgeKind::Derives,
        EdgeKind::Cites,
        EdgeKind::Implements,
        EdgeKind::Instantiates,
        EdgeKind::Evaluates,
        EdgeKind::Solves,
        EdgeKind::Binds,
        EdgeKind::Realizes,
        EdgeKind::Attests,
        EdgeKind::Waives,
    ];

    /// The spec name of the edge (`derives`, `cites`, ...).
    pub fn name(self) -> &'static str {
        match self {
            EdgeKind::Derives => "derives",
            EdgeKind::Cites => "cites",
            EdgeKind::Implements => "implements",
            EdgeKind::Instantiates => "instantiates",
            EdgeKind::Evaluates => "evaluates",
            EdgeKind::Solves => "solves",
            EdgeKind::Binds => "binds",
            EdgeKind::Realizes => "realizes",
            EdgeKind::Attests => "attests",
            EdgeKind::Waives => "waives",
        }
    }

    /// Whether an edge of this kind may run from a node of kind `from` to one of kind `to`.
    pub fn allows(self, from: NodeKind, to: NodeKind) -> bool {
        use NodeKind as N;
        match self {
            EdgeKind::Derives => from == N::Param && to == N::Param,
            EdgeKind::Cites => {
                matches!(from, N::Param | N::Table | N::Component) && to == N::Citation
            }
            EdgeKind::Implements => from == N::Component && to == N::Interface,
            EdgeKind::Instantiates => from == N::Part && matches!(to, N::Component | N::Part),
            EdgeKind::Evaluates => matches!(from, N::Check | N::Require | N::Fit),
            EdgeKind::Solves => from == N::SolverResult,
            EdgeKind::Binds => from == N::Param && to == N::Binding,
            EdgeKind::Realizes => from == N::NativeFile && to == N::Param,
            EdgeKind::Attests => from == N::Attestation,
            EdgeKind::Waives => from == N::Waiver,
        }
    }
}

impl fmt::Display for EdgeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// Index of a node within one [`Graph`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeId(usize);

/// A graph node: an IR entity with a canonical path and an optional digest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    /// What kind of entity this is.
    pub kind: NodeKind,
    /// Canonical path of the entity (ir.md 4).
    pub path: String,
    /// Closure digest, hex, when known.
    pub digest: Option<String>,
}

/// Errors raised while building a [`Graph`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphError {
    /// A node id does not belong to this graph.
    UnknownNode(NodeId),
    /// The edge kind does not allow these endpoint kinds.
    BadEdge {
        /// Edge kind attempted.
        edge: EdgeKind,
        /// Source node kind.
        from: NodeKind,
        /// Target node kind.
        to: NodeKind,
    },
}

impl fmt::Display for GraphError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GraphError::UnknownNode(n) => write!(f, "unknown node #{}", n.0),
            GraphError::BadEdge { edge, from, to } => {
                write!(f, "edge `{edge}` cannot run from {from:?} to {to:?}")
            }
        }
    }
}

impl std::error::Error for GraphError {}

/// The obligation graph: IR entities as nodes, typed edges between them.
#[derive(Debug, Default, Clone)]
pub struct Graph {
    nodes: Vec<Node>,
    edges: Vec<(NodeId, EdgeKind, NodeId)>,
}

impl Graph {
    /// Create an empty graph.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a node and return its id.
    pub fn add_node(&mut self, kind: NodeKind, path: &str, digest: Option<&str>) -> NodeId {
        let id = NodeId(self.nodes.len());
        trace!(?kind, path, "graph: add node");
        self.nodes.push(Node {
            kind,
            path: path.to_owned(),
            digest: digest.map(str::to_owned),
        });
        id
    }

    /// Look up a node by id.
    pub fn node(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(id.0)
    }

    /// Add an edge, validating the endpoint kinds against the spec table; duplicates are ignored.
    pub fn add_edge(&mut self, from: NodeId, kind: EdgeKind, to: NodeId) -> Result<(), GraphError> {
        let f = self.node(from).ok_or(GraphError::UnknownNode(from))?.kind;
        let t = self.node(to).ok_or(GraphError::UnknownNode(to))?.kind;
        if !kind.allows(f, t) {
            warn!(edge = kind.name(), ?f, ?t, "graph: rejected edge");
            return Err(GraphError::BadEdge {
                edge: kind,
                from: f,
                to: t,
            });
        }
        let e = (from, kind, to);
        if !self.edges.contains(&e) {
            debug!(
                edge = kind.name(),
                from = from.0,
                to = to.0,
                "graph: add edge"
            );
            self.edges.push(e);
        }
        Ok(())
    }

    /// Number of nodes.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// All edges of one kind as (from, to) pairs.
    pub fn edges_of(&self, kind: EdgeKind) -> Vec<(NodeId, NodeId)> {
        self.edges
            .iter()
            .filter(|e| e.1 == kind)
            .map(|e| (e.0, e.2))
            .collect()
    }

    /// Direct successors of `from` along edges of `kind`.
    pub fn successors(&self, from: NodeId, kind: EdgeKind) -> Vec<NodeId> {
        self.edges
            .iter()
            .filter(|e| e.0 == from && e.1 == kind)
            .map(|e| e.2)
            .collect()
    }
}

/// Obligation kind (obligations.md 2.2); the prefix of an obligation id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ObligationKind {
    /// Source sign-off of an `ots` component.
    Ots,
    /// Sign-off of a fit's result.
    Fit,
    /// Sign-off of a check's result.
    Check,
    /// Sign-off of a `require` that holds.
    Require,
    /// Solver result is current.
    Solver,
    /// A cited file is unchanged since acknowledged.
    Citation,
    /// CAD file reflects design.
    Sync,
    /// A waiver was reviewed against its targets' current digests.
    Waiver,
}

impl ObligationKind {
    /// All kinds.
    pub const ALL: [ObligationKind; 8] = [
        ObligationKind::Ots,
        ObligationKind::Fit,
        ObligationKind::Check,
        ObligationKind::Require,
        ObligationKind::Solver,
        ObligationKind::Citation,
        ObligationKind::Sync,
        ObligationKind::Waiver,
    ];

    /// The id prefix (`ots`, `fit`, ...).
    pub fn name(self) -> &'static str {
        match self {
            ObligationKind::Ots => "ots",
            ObligationKind::Fit => "fit",
            ObligationKind::Check => "check",
            ObligationKind::Require => "require",
            ObligationKind::Solver => "solver",
            ObligationKind::Citation => "citation",
            ObligationKind::Sync => "sync",
            ObligationKind::Waiver => "waiver",
        }
    }

    /// Parse an id prefix.
    pub fn from_name(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.name() == s)
    }

    /// Whether this kind is recomputed every run and never stale (check, require, fit; obligations.md 2.3).
    pub fn is_recomputed(self) -> bool {
        matches!(
            self,
            ObligationKind::Check | ObligationKind::Require | ObligationKind::Fit
        )
    }
}

/// Error parsing an obligation id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdError {
    /// No `:` separator or an empty path.
    Malformed(String),
    /// The kind prefix is not a known obligation kind.
    UnknownKind(String),
}

impl fmt::Display for IdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IdError::Malformed(s) => write!(f, "malformed obligation id `{s}`"),
            IdError::UnknownKind(s) => write!(f, "unknown obligation kind `{s}`"),
        }
    }
}

impl std::error::Error for IdError {}

/// Obligation id `<kind>:<canonical path>` (obligations.md 2.2).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ObligationId {
    kind: ObligationKind,
    path: String,
}

impl ObligationId {
    /// Build an id from a kind and canonical path.
    pub fn new(kind: ObligationKind, path: impl Into<String>) -> Self {
        Self {
            kind,
            path: path.into(),
        }
    }

    /// The obligation kind.
    pub fn kind(&self) -> ObligationKind {
        self.kind
    }

    /// The canonical path.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Parse `<kind>:<path>`; the path may itself contain `:` (e.g. `acme::X`).
    pub fn parse(s: &str) -> Result<Self, IdError> {
        let (k, p) = s
            .split_once(':')
            .ok_or_else(|| IdError::Malformed(s.to_owned()))?;
        if p.is_empty() {
            return Err(IdError::Malformed(s.to_owned()));
        }
        let kind =
            ObligationKind::from_name(k).ok_or_else(|| IdError::UnknownKind(k.to_owned()))?;
        Ok(Self::new(kind, p))
    }
}

impl fmt::Display for ObligationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.kind.name(), self.path)
    }
}

/// Obligation state (obligations.md 2.3); exactly one applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ObligationState {
    /// Discharged and current.
    Discharged,
    /// Upstream changed since discharge.
    Stale,
    /// Evaluated and does not hold.
    Violated,
    /// Evaluation could not decide.
    Undecided,
    /// Never discharged.
    Open,
    /// Diagnostic suppressed by `waive` or `allow`.
    Waived,
}

/// Verdict of a fresh evaluation, for recomputed kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Holds over every tolerance and alternate.
    Holds,
    /// Violated somewhere.
    Violated,
    /// Cannot be decided.
    Undecided,
}

/// Everything needed to classify one obligation.
#[derive(Debug, Clone, Copy)]
pub struct Evidence<'a> {
    /// Obligation kind.
    pub kind: ObligationKind,
    /// Fresh verdict; consulted only for recomputed kinds.
    pub verdict: Option<Verdict>,
    /// Closure digest recorded at discharge, if any.
    pub recorded: Option<&'a str>,
    /// Current closure digest.
    pub current: &'a str,
    /// Whether the diagnostic is suppressed by `waive` or `allow`.
    pub suppressed: bool,
}

/// Classify an obligation into exactly one state.
///
/// Recomputed kinds (check, require, fit) never go stale. `waived` never applies to a `require`.
pub fn classify(ev: &Evidence<'_>) -> ObligationState {
    use ObligationState as S;
    let state = if ev.suppressed && ev.kind != ObligationKind::Require {
        S::Waived
    } else if ev.kind.is_recomputed() {
        match ev.verdict {
            Some(Verdict::Holds) => S::Discharged,
            Some(Verdict::Violated) => S::Violated,
            Some(Verdict::Undecided) | None => S::Undecided,
        }
    } else {
        match ev.recorded {
            None => S::Open,
            Some(r) if r == ev.current => S::Discharged,
            Some(_) => S::Stale,
        }
    };
    debug!(kind = ev.kind.name(), ?state, "classified obligation");
    state
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(kind: ObligationKind) -> Evidence<'static> {
        Evidence {
            kind,
            verdict: None,
            recorded: None,
            current: "aa",
            suppressed: false,
        }
    }

    // frob:tests crates/cadetta-obligations/src/graph.rs::EdgeKind
    #[test]
    fn all_ten_edges_buildable() {
        let mut g = Graph::new();
        let p1 = g.add_node(NodeKind::Param, "a::P1", Some("01"));
        let p2 = g.add_node(NodeKind::Param, "a::P2", None);
        let cit = g.add_node(NodeKind::Citation, "a::C", None);
        let tab = g.add_node(NodeKind::Table, "a::T", None);
        let comp = g.add_node(NodeKind::Component, "a::Comp", None);
        let ifc = g.add_node(NodeKind::Interface, "a::I", None);
        let part = g.add_node(NodeKind::Part, "a::Part", None);
        let chk = g.add_node(NodeKind::Check, "a::chk", None);
        let sol = g.add_node(NodeKind::SolverResult, "a::sol", None);
        let bind = g.add_node(NodeKind::Binding, "a::b", None);
        let nat = g.add_node(NodeKind::NativeFile, "x.SLDPRT", None);
        let att = g.add_node(NodeKind::Attestation, "att", None);
        let wv = g.add_node(NodeKind::Waiver, "w", None);
        for (f, k, t) in [
            (p1, EdgeKind::Derives, p2),
            (p1, EdgeKind::Cites, cit),
            (tab, EdgeKind::Cites, cit),
            (comp, EdgeKind::Cites, cit),
            (comp, EdgeKind::Implements, ifc),
            (part, EdgeKind::Instantiates, comp),
            (chk, EdgeKind::Evaluates, p1),
            (sol, EdgeKind::Solves, p1),
            (p1, EdgeKind::Binds, bind),
            (nat, EdgeKind::Realizes, p1),
            (att, EdgeKind::Attests, chk),
            (wv, EdgeKind::Waives, chk),
        ] {
            assert_eq!(g.add_edge(f, k, t), Ok(()), "{k}");
        }
        for k in EdgeKind::ALL {
            assert!(!g.edges_of(k).is_empty(), "{k}");
        }
        assert_eq!(g.successors(p1, EdgeKind::Derives), vec![p2]);
        assert_eq!(g.node_count(), 13);
    }

    // frob:tests crates/cadetta-obligations/src/graph.rs::Graph.add_edge
    #[test]
    fn bad_edges_rejected_and_duplicates_ignored() {
        let mut g = Graph::new();
        let p = g.add_node(NodeKind::Param, "p", None);
        let c = g.add_node(NodeKind::Citation, "c", None);
        assert!(matches!(
            g.add_edge(c, EdgeKind::Derives, p),
            Err(GraphError::BadEdge { .. })
        ));
        assert_eq!(
            g.add_edge(p, EdgeKind::Cites, NodeId(99)),
            Err(GraphError::UnknownNode(NodeId(99)))
        );
        g.add_edge(p, EdgeKind::Cites, c).unwrap();
        g.add_edge(p, EdgeKind::Cites, c).unwrap();
        assert_eq!(g.edges_of(EdgeKind::Cites).len(), 1);
    }

    // frob:tests crates/cadetta-obligations/src/graph.rs::classify
    #[test]
    fn classify_attested_kinds() {
        let mut e = ev(ObligationKind::Ots);
        assert_eq!(classify(&e), ObligationState::Open);
        e.recorded = Some("aa");
        assert_eq!(classify(&e), ObligationState::Discharged);
        e.recorded = Some("bb");
        assert_eq!(classify(&e), ObligationState::Stale);
        e.suppressed = true;
        assert_eq!(classify(&e), ObligationState::Waived);
    }

    // frob:tests crates/cadetta-obligations/src/graph.rs::classify
    #[test]
    fn recomputed_kinds_never_stale() {
        for k in [
            ObligationKind::Check,
            ObligationKind::Require,
            ObligationKind::Fit,
        ] {
            let mut e = ev(k);
            e.recorded = Some("old");
            for (v, want) in [
                (Some(Verdict::Holds), ObligationState::Discharged),
                (Some(Verdict::Violated), ObligationState::Violated),
                (Some(Verdict::Undecided), ObligationState::Undecided),
                (None, ObligationState::Undecided),
            ] {
                e.verdict = v;
                assert_eq!(classify(&e), want);
            }
        }
    }

    // frob:tests crates/cadetta-obligations/src/graph.rs::classify
    #[test]
    fn require_never_waived() {
        let mut e = ev(ObligationKind::Require);
        e.verdict = Some(Verdict::Violated);
        e.suppressed = true;
        assert_eq!(classify(&e), ObligationState::Violated);
        e.kind = ObligationKind::Check;
        assert_eq!(classify(&e), ObligationState::Waived);
    }

    // frob:tests crates/cadetta-obligations/src/graph.rs::ObligationId
    // frob:tests crates/cadetta-obligations/src/graph.rs::ObligationId.path
    #[test]
    fn ids_roundtrip_and_include_waiver() {
        for k in ObligationKind::ALL {
            let id = ObligationId::new(k, "acme.rover::Axle.seat");
            assert_eq!(id.path(), "acme.rover::Axle.seat");
            assert_eq!(ObligationId::parse(&id.to_string()), Ok(id));
        }
        assert_eq!(
            ObligationId::parse("waiver:W001").unwrap().kind(),
            ObligationKind::Waiver
        );
        assert_eq!(
            ObligationId::parse("nope:x"),
            Err(IdError::UnknownKind("nope".into()))
        );
        assert!(ObligationId::parse("fit:").is_err());
        assert!(ObligationId::parse("fit").is_err());
    }
}
