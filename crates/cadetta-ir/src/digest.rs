//! BLAKE3 own and closure (Merkle) digests (`docs/spec/ir.md` 4).
//!
//! Decisions made here (written into ir.md 4): the canonical encoding is tagged and
//! length-prefixed, so field boundaries are unambiguous; spans, doc comments,
//! `Value.authored` (display text of the value), and the presentational attributes
//! `display`, `emit`, `allow` and `lifecycle` are excluded and every other attribute is
//! semantic; verdicts, `conservative` and `waived_for` of a check are results, not
//! inputs, and are excluded; a closure hashes the own digest then the dep closure digests
//! in dep path order; an instance closure hashes the member closure, the instance path text
//! and, for an interface-typed slot, the slot part's own digest. Part arguments are graph
//! nodes keyed `<slot>.<arg>`, so a member that reads one argument depends on that argument
//! only.
// frob:ticket 01M4SPWQED1EN19NWQJE5NT0J4

use std::collections::{BTreeMap, BTreeSet};

use tracing::{debug, trace};

use crate::entities::Require;
use crate::model::{
    Affine, Binding, Bound, Check, CitationType, CiteField, CiteRef, Component, Digest, Exact, Fit,
    Locator, Member, Nominal, Part, Project, Provenance, Solver, Value,
};

/// Attribute keys that never enter a digest (ir.md 4).
pub const PRESENTATIONAL_ATTRS: [&str; 4] = ["display", "emit", "allow", "lifecycle"];

/// A digest computation failure.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DigestError {
    /// A node reads a node that is not in the graph.
    #[error("`{node}` depends on unknown node `{dep}`")]
    MissingDep {
        /// The reading node.
        node: String,
        /// The unknown dependency.
        dep: String,
    },
    /// The dependency graph has a cycle (D003).
    #[error("dependency cycle through `{0}`")]
    Cycle(String),
    /// A queried node is not in the graph.
    #[error("unknown node `{0}`")]
    Unknown(String),
}

/// Digest arbitrary bytes as lowercase BLAKE3 hex.
pub fn digest_bytes(bytes: &[u8]) -> Digest {
    Digest(blake3::hash(bytes).to_hex().to_string())
}

/// Canonical field encoder: every field is length-prefixed so boundaries are unambiguous.
struct Enc(blake3::Hasher);

impl Enc {
    fn new(tag: &str) -> Self {
        let mut e = Enc(blake3::Hasher::new());
        e.str(tag);
        e
    }

    fn str(&mut self, s: &str) -> &mut Self {
        self.0.update(&(s.len() as u64).to_le_bytes());
        self.0.update(s.as_bytes());
        self
    }

    fn num(&mut self, n: u64) -> &mut Self {
        self.0.update(&n.to_le_bytes());
        self
    }

    fn flag(&mut self, b: bool) -> &mut Self {
        self.0.update(&[u8::from(b)]);
        self
    }

    fn opt<T>(&mut self, v: &Option<T>, f: impl FnOnce(&mut Self, &T)) -> &mut Self {
        match v {
            Some(x) => {
                self.flag(true);
                f(self, x);
            }
            None => {
                self.flag(false);
            }
        }
        self
    }

    fn digest(&mut self, d: &Digest) -> &mut Self {
        self.str(&d.0)
    }

    fn finish(&self) -> Digest {
        Digest(self.0.finalize().to_hex().to_string())
    }
}

fn exact(e: &mut Enc, x: &Exact) {
    e.str(&x.0.rational().numer().to_string())
        .str(&x.0.rational().denom().to_string())
        .num(x.0.pi_exp() as u32 as u64);
}

fn affine(e: &mut Enc, a: &Affine) {
    e.num(a.terms.len() as u64);
    for t in &a.terms {
        e.str(&t.var.to_string());
        exact(e, &t.lo);
        exact(e, &t.hi);
    }
}

fn bound(e: &mut Enc, b: &Bound) {
    e.str(&format!("{:?}", b.op));
    e.opt(&b.lo, exact);
    e.opt(&b.hi, exact);
}

fn value(e: &mut Enc, v: &Value) {
    match &v.nominal {
        Nominal::Exact(x) => {
            e.str("exact");
            exact(e, x);
        }
        Nominal::Inexact(i) => {
            e.str("inexact")
                .num(i.f64.to_bits())
                .str(&i.root.to_string());
        }
    }
    e.str(&v.dim.dimension.render())
        .num(v.dim.angle as u32 as u64);
    e.num(v.angle as u32 as u64);
    e.opt(&v.tol, affine);
    e.flag(v.conservative);
    e.opt(&v.drawn, exact);
    e.flag(v.exact);
    e.opt(&v.reason, |e, s| {
        e.str(s);
    });
    e.opt(&v.unit_assumed, |e, s| {
        e.str(s);
    });
}

/// A use-site reference: the cite item path (or built-in form, note text) and locator only.
fn cite_ref(e: &mut Enc, c: &CiteRef) {
    e.opt(&c.cite, |e, s| {
        e.str(s);
    });
    e.opt(&c.builtin, |e, b| {
        e.str(b.name());
    });
    e.opt(&c.text, |e, s| {
        e.str(s);
    });
    e.opt(&c.locator, |e, s| {
        e.str(s);
    });
}

/// Digest of a `cite` item: its type, fields in name order, and the file digest (`absent` when
/// the named file is missing). The item's path is not part of it (ir.md 3).
// frob:ticket 01M4VB62VJQV9AKSSQME8NWW6Y
pub fn citation_digest(
    ty: CitationType,
    fields: &BTreeMap<String, CiteField>,
    file_digest: Option<&Digest>,
) -> Digest {
    let mut e = Enc::new("citation");
    e.str(ty.name());
    e.num(fields.len() as u64);
    for (k, v) in fields {
        e.str(k);
        match v {
            CiteField::Text(t) => {
                e.str("text").str(t);
            }
            CiteField::List(l) => {
                e.str("list").num(l.len() as u64);
                for item in l {
                    e.str(item);
                }
            }
        }
    }
    match file_digest {
        Some(d) => {
            e.digest(d);
        }
        None => {
            e.str("absent");
        }
    }
    e.finish()
}

fn attrs(e: &mut Enc, a: &BTreeMap<String, String>) {
    let sem: Vec<_> = a
        .iter()
        .filter(|(k, _)| !PRESENTATIONAL_ATTRS.contains(&k.as_str()))
        .collect();
    e.num(sem.len() as u64);
    for (k, v) in sem {
        e.str(k).str(v);
    }
}

/// Own digest of a member: path, kind, value, expression, citation and semantic attrs.
pub fn member_digest(m: &Member) -> Digest {
    let mut e = Enc::new("member");
    e.str(&m.path.to_string())
        .str(&format!("{:?}", m.kind))
        .flag(m.analysis)
        .flag(m.private);
    e.opt(&m.value, value);
    e.opt(&m.bound, bound);
    e.opt(&m.expr, |e, s| {
        e.str(s);
    });
    e.opt(&m.role, |e, s| {
        e.str(s);
    });
    e.opt(&m.citation, cite_ref);
    e.num(m.was.len() as u64);
    for w in &m.was {
        e.str(w);
    }
    attrs(&mut e, &m.attrs);
    match &m.provenance {
        Provenance::Authored => {
            e.str("authored");
        }
        Provenance::Derived => {
            e.str("derived");
        }
        Provenance::Imported {
            adapter,
            file,
            native_name,
        } => {
            e.str("imported").str(adapter).str(file).str(native_name);
        }
    }
    e.opt(&m.variants, |e, vs| {
        e.num(vs.len() as u64);
        for (k, v) in vs {
            e.str(k);
            value(e, v);
        }
    });
    let d = e.finish();
    trace!(path = %m.path, digest = %d.0, "digest: member own");
    d
}

// frob:ticket 01M4VQXPDA4TEKE455FNYB5TXC
/// Encodes an op chain: kind and argument member paths.
fn ops(e: &mut Enc, ops: &[crate::geometry::Op]) {
    use crate::geometry::Op;
    e.num(ops.len() as u64);
    for o in ops {
        match o {
            Op::Offset(a) => {
                e.str("offset");
                for p in [&a.x, &a.y, &a.z] {
                    e.opt(p, |e, p| {
                        e.str(&p.to_string());
                    });
                }
            }
            Op::Rotate { axis, angle } => {
                e.str("rotate")
                    .str(&format!("{axis:?}"))
                    .str(&angle.to_string());
            }
            Op::Flip { axis } => {
                e.str("flip").str(&format!("{axis:?}"));
            }
        }
    }
}

// frob:ticket 01M4VQXPDA4TEKE455FNYB5TXC
/// Own digest of a datum: path, parent and op chain (per-alternate chains included).
pub fn datum_digest(d: &crate::geometry::Datum) -> Digest {
    let mut e = Enc::new("datum");
    e.str(&d.path.to_string()).str(&d.parent.to_string());
    ops(&mut e, &d.ops);
    e.opt(&d.variants, |e, vs| {
        e.num(vs.len() as u64);
        for (k, v) in vs {
            e.str(k);
            ops(e, v);
        }
    });
    e.finish()
}

// frob:ticket 01M4VQXPDA4TEKE455FNYB5TXC
/// Own digest of a point: path, datum and coordinate values.
pub fn point_digest(p: &crate::geometry::Point) -> Digest {
    let mut e = Enc::new("point");
    e.str(&p.path.to_string()).str(&p.datum.to_string());
    e.num(p.coords.len() as u64);
    for c in &p.coords {
        value(&mut e, c);
    }
    e.finish()
}

// frob:ticket 01M4VQXPDA4TEKE455FNYB5TXC
/// Own digest of a placement: path, part, datum, target and op chain.
pub fn placement_digest(p: &crate::geometry::Placement) -> Digest {
    let mut e = Enc::new("placement");
    e.str(&p.path.to_string())
        .str(&p.part.to_string())
        .str(&p.datum.to_string())
        .str(&p.at.to_string());
    ops(&mut e, &p.ops);
    e.opt(&p.variants, |e, vs| {
        e.num(vs.len() as u64);
        for (k, v) in vs {
            e.str(k);
            ops(e, v);
        }
    });
    e.finish()
}

/// Own digest of a part: type, count, primary, alternates (as a set) and arguments.
pub fn part_digest(p: &Part) -> Digest {
    let mut e = Enc::new("part");
    e.str(&p.path.to_string()).str(&p.type_name);
    e.opt(&p.count, |e, n| {
        e.num(u64::from(*n));
    });
    e.str(&p.primary);
    let alts: BTreeSet<_> = p.alternates.iter().collect();
    e.num(alts.len() as u64);
    for a in alts {
        e.str(a);
    }
    e.num(p.args.len() as u64);
    for (k, v) in &p.args {
        e.str(k).str(v);
    }
    e.finish()
}

/// Own digest of one part argument (name and expression text).
pub fn arg_digest(name: &str, expr: &str) -> Digest {
    Enc::new("arg").str(name).str(expr).finish()
}

/// Own digest of a fit.
pub fn fit_digest(f: &Fit) -> Digest {
    let mut e = Enc::new("fit");
    e.str(&f.path.to_string())
        .str(&f.hole.to_string())
        .str(&f.shaft.to_string());
    value(&mut e, &f.clearance);
    e.str(&f.fit_type);
    e.opt(&f.required, bound);
    e.finish()
}

/// Own digest of a lookup table: its path, columns, rows and citation reference.
// frob:ticket 01M4Y9Z3KDPFRNGQR1KTNGHA7Q
pub fn table_digest(t: &crate::entities::Table) -> Digest {
    use crate::entities::Cell;
    let cell = |e: &mut Enc, c: &Cell| match c {
        Cell::Value(v) => {
            e.str("value");
            value(e, v);
        }
        Cell::Variant(s) => {
            e.str("variant").str(s);
        }
    };
    let mut e = Enc::new("table");
    e.str(&t.path.to_string());
    e.str(&t.key.name).str(&t.key.type_name);
    e.num(t.cols.len() as u64);
    for c in &t.cols {
        e.str(&c.name).str(&c.type_name);
    }
    e.num(t.rows.len() as u64);
    for r in &t.rows {
        cell(&mut e, &r.key);
        e.num(r.cells.len() as u64);
        for c in &r.cells {
            cell(&mut e, c);
        }
    }
    e.opt(&t.citation, cite_ref);
    e.finish()
}

/// Own digest of a solver, including its recorded result digest (ir.md 4).
pub fn solver_digest(s: &Solver) -> Digest {
    let mut e = Enc::new("solver");
    e.str(&s.path.to_string()).str(&s.script);
    e.num(s.inputs.len() as u64);
    for (k, v) in &s.inputs {
        e.str(k).str(v);
    }
    e.num(s.outputs.len() as u64);
    for (k, v) in &s.outputs {
        e.str(k).str(v);
    }
    e.opt(&s.result_digest, |e, d| {
        e.digest(d);
    });
    e.finish()
}

/// Own digest of a check: its path and expression; verdicts are results, not inputs.
pub fn check_digest(c: &Check) -> Digest {
    Enc::new("check")
        .str(&c.path.to_string())
        .str(&c.expr)
        .finish()
}

/// Own digest of a require: its path, expression, message and citation; verdicts are results.
// frob:ticket 01M4YKTX2YMSXVE4CK0JAVMC41
pub fn require_digest(r: &Require) -> Digest {
    let mut e = Enc::new("require");
    e.str(&r.path.to_string()).str(&r.expr);
    e.opt(&r.message, |e, s| {
        e.str(s);
    });
    e.opt(&r.citation, cite_ref);
    e.finish()
}

fn locator(e: &mut Enc, l: &Locator) {
    e.str(&l.locator)
        .str(&format!("{:?}{:?}", l.kind, l.mode))
        .str(&l.native_unit);
    for o in [&l.ref_to, &l.for_alternate, &l.config, &l.component] {
        e.opt(o, |e, s| {
            e.str(s);
        });
    }
    e.opt(&l.sign, |e, s| {
        e.str(&format!("{s:?}"));
    });
}

/// Own digest of a binding.
pub fn binding_digest(b: &Binding) -> Digest {
    let mut e = Enc::new("binding");
    e.str(&b.path.to_string()).str(&b.adapter);
    e.num(b.locators.len() as u64);
    for l in &b.locators {
        locator(&mut e, l);
    }
    e.finish()
}

/// Own digest of a component over its header and the own digests of its children,
/// sorted by path so declaration order never matters.
pub fn component_digest(c: &Component) -> Digest {
    let mut e = Enc::new("component");
    e.str(&c.path.to_string()).str(&format!("{:?}", c.class));
    e.opt(&c.params_sig, |e, s| {
        e.str(s);
    });
    let imp: BTreeSet<_> = c.implements.iter().collect();
    e.num(imp.len() as u64);
    for i in imp {
        e.str(i);
    }
    e.opt(&c.citation, cite_ref);
    attrs(&mut e, &c.attrs);
    let mut kids: Vec<(String, Digest)> = Vec::new();
    for m in &c.members {
        kids.push((m.path.to_string(), member_digest(m)));
    }
    for p in &c.parts {
        kids.push((p.path.to_string(), part_digest(p)));
    }
    for f in &c.fits {
        kids.push((f.path.to_string(), fit_digest(f)));
    }
    // frob:ticket 01M4VQXPDA4TEKE455FNYB5TXC
    for d in &c.datums {
        kids.push((d.path.to_string(), datum_digest(d)));
    }
    for p in &c.points {
        kids.push((p.path.to_string(), point_digest(p)));
    }
    for p in &c.placements {
        kids.push((p.path.to_string(), placement_digest(p)));
    }
    kids.sort_by(|a, b| a.0.cmp(&b.0));
    e.num(kids.len() as u64);
    for (k, d) in &kids {
        e.str(k).digest(d);
    }
    e.finish()
}

/// Own digest of a whole project; node digests are folded in sorted order.
pub fn project_digest(p: &Project) -> Digest {
    let mut e = Enc::new("project");
    e.str(&p.schema)
        .str(&format!("{:?}", p.flavour))
        .str(&p.package.name)
        .str(&p.package.version);
    e.num(p.modules.len() as u64);
    for m in &p.modules {
        e.str(m);
    }
    e.num(p.dependencies.len() as u64);
    for d in &p.dependencies {
        e.str(&d.name).str(&d.version).str(&d.source).str(&d.digest);
    }
    let mut kids: Vec<(String, Digest)> = Vec::new();
    kids.extend(
        p.components
            .iter()
            .map(|c| (c.path.to_string(), component_digest(c))),
    );
    kids.extend(
        p.checks
            .iter()
            .map(|c| (c.path.to_string(), check_digest(c))),
    );
    kids.extend(
        p.requires
            .iter()
            .map(|r| (r.path.to_string(), require_digest(r))),
    );
    kids.extend(
        p.solvers
            .iter()
            .map(|s| (s.path.to_string(), solver_digest(s))),
    );
    kids.extend(
        p.bindings
            .iter()
            .map(|b| (format!("{}@{}", b.path, b.adapter), binding_digest(b))),
    );
    let mut cites: Vec<(String, Digest)> = p
        .citations
        .iter()
        .map(|c| (c.path.clone(), c.digest.clone()))
        .collect();
    kids.append(&mut cites);
    kids.sort_by(|a, b| a.0.cmp(&b.0));
    e.num(kids.len() as u64);
    for (k, d) in &kids {
        e.str(k).digest(d);
    }
    e.finish()
}

#[derive(Debug, Clone)]
struct Node {
    own: Digest,
    deps: Vec<String>,
}

/// The dependency graph of own digests from which closure digests are computed.
#[derive(Debug, Clone, Default)]
pub struct DigestGraph {
    nodes: BTreeMap<String, Node>,
}

impl DigestGraph {
    /// An empty graph.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a node by key with its own digest and the keys it reads.
    pub fn add_node(&mut self, key: impl Into<String>, own: Digest, deps: Vec<String>) {
        let key = key.into();
        debug!(%key, deps = deps.len(), "digest: node added");
        self.nodes.insert(key, Node { own, deps });
    }

    /// Add a member keyed by its path, reading `member.deps`.
    pub fn add_member(&mut self, m: &Member) {
        let deps = m.deps.iter().map(ToString::to_string).collect();
        self.add_node(m.path.to_string(), member_digest(m), deps);
    }

    /// Add one input node `<component>.<param>` per name in `params_sig` (members read them).
    pub fn add_params(&mut self, c: &Component) {
        let Some(sig) = &c.params_sig else { return };
        for name in sig.split(',').map(str::trim).filter(|n| !n.is_empty()) {
            let key = format!("{}.{name}", c.path);
            let mut e = Enc::new("param");
            e.str(&key);
            self.add_node(key, e.finish(), Vec::new());
        }
    }

    /// Add a fit keyed by its path plus its `<fit>.hole` and `<fit>.shaft` accessor nodes, which
    /// derive from it. The accessors are keyed under the fit, never under the operand members
    /// (`Fit.hole`/`Fit.shaft`), which are real members with their own nodes.
    pub fn add_fit(&mut self, f: &Fit) {
        let own = fit_digest(f);
        let key = f.path.to_string();
        self.add_node(key.clone(), own.clone(), Vec::new());
        for side in ["hole", "shaft"] {
            let node = format!("{key}.{side}");
            let mut e = Enc::new("fit-side");
            e.str(&node).digest(&own);
            self.add_node(node, e.finish(), vec![key.clone()]);
        }
    }

    /// Add a solver keyed by its path, reading `solver.deps`.
    pub fn add_solver(&mut self, s: &Solver) {
        let deps = s.deps.iter().map(ToString::to_string).collect();
        self.add_node(s.path.to_string(), solver_digest(s), deps);
    }

    // frob:ticket 01M4VQXPDA4TEKE455FNYB5TXC
    /// Add a datum keyed by its path; it depends on its parent and every op argument member.
    pub fn add_datum(&mut self, d: &crate::geometry::Datum) {
        let mut deps = frame_dep(&d.parent);
        deps.extend(op_paths(&d.ops));
        self.add_node(d.path.to_string(), datum_digest(d), deps);
    }

    // frob:ticket 01M4VQXPDA4TEKE455FNYB5TXC
    /// Add a placement keyed by its path; it depends on its target and every op argument member.
    pub fn add_placement(&mut self, p: &crate::geometry::Placement) {
        let mut deps = frame_dep(&p.at);
        deps.extend(op_paths(&p.ops));
        self.add_node(p.path.to_string(), placement_digest(p), deps);
    }

    /// Add a part slot under `slot` plus one node `<slot>.<arg>` per argument.
    pub fn add_part(&mut self, slot: &str, p: &Part) {
        self.add_node(slot, part_digest(p), Vec::new());
        for (k, v) in &p.args {
            self.add_node(format!("{slot}.{k}"), arg_digest(k, v), Vec::new());
        }
    }

    /// The own digest of a node.
    pub fn own(&self, key: &str) -> Result<&Digest, DigestError> {
        self.nodes
            .get(key)
            .map(|n| &n.own)
            .ok_or_else(|| DigestError::Unknown(key.to_owned()))
    }

    /// The Merkle closure of `key`: its own digest plus the closures of its deps in path order.
    pub fn closure(&self, key: &str) -> Result<Digest, DigestError> {
        let mut memo = BTreeMap::new();
        let mut visiting = BTreeSet::new();
        self.closure_in(key, &mut memo, &mut visiting)
    }

    // frob:ticket 01M4ST8ERHSJXGDXNH75ZCX6H7
    /// Own digests of every transitive dep of `key` (not `key` itself), sorted and unique.
    ///
    /// This is the basis of `why --stale`: an attestation stores it (obligations.md 3).
    pub fn dep_owns(&self, key: &str) -> Result<Vec<Digest>, DigestError> {
        let root = self
            .nodes
            .get(key)
            .ok_or_else(|| DigestError::Unknown(key.to_owned()))?;
        let mut seen: BTreeSet<&str> = BTreeSet::new();
        let mut stack: Vec<&str> = root.deps.iter().map(String::as_str).collect();
        let mut owns: BTreeSet<String> = BTreeSet::new();
        while let Some(k) = stack.pop() {
            if k == key || !seen.insert(k) {
                continue;
            }
            let n = self.nodes.get(k).ok_or_else(|| DigestError::MissingDep {
                node: key.to_owned(),
                dep: k.to_owned(),
            })?;
            owns.insert(n.own.0.clone());
            stack.extend(n.deps.iter().map(String::as_str));
        }
        trace!(%key, deps = owns.len(), "digest: dep own digests");
        Ok(owns.into_iter().map(Digest).collect())
    }

    /// Closures of every node, computed once with shared memoization.
    pub fn closures(&self) -> Result<BTreeMap<String, Digest>, DigestError> {
        let mut memo = BTreeMap::new();
        let mut visiting = BTreeSet::new();
        for k in self.nodes.keys() {
            self.closure_in(k, &mut memo, &mut visiting)?;
        }
        Ok(memo)
    }

    fn closure_in(
        &self,
        key: &str,
        memo: &mut BTreeMap<String, Digest>,
        visiting: &mut BTreeSet<String>,
    ) -> Result<Digest, DigestError> {
        if let Some(d) = memo.get(key) {
            return Ok(d.clone());
        }
        let node = self
            .nodes
            .get(key)
            .ok_or_else(|| DigestError::Unknown(key.to_owned()))?;
        if !visiting.insert(key.to_owned()) {
            debug!(%key, "digest: cycle");
            return Err(DigestError::Cycle(key.to_owned()));
        }
        let deps: BTreeSet<&String> = node.deps.iter().collect();
        let mut e = Enc::new("closure");
        e.digest(&node.own).num(deps.len() as u64);
        for dep in deps {
            if !self.nodes.contains_key(dep) {
                return Err(DigestError::MissingDep {
                    node: key.to_owned(),
                    dep: dep.clone(),
                });
            }
            let d = self.closure_in(dep, memo, visiting)?;
            e.digest(&d);
        }
        visiting.remove(key);
        let d = e.finish();
        trace!(%key, digest = %d.0, "digest: closure");
        memo.insert(key.to_owned(), d.clone());
        Ok(d)
    }

    /// Closure of one instance of a definition member, e.g. `Rover/wheel[2].hub_bore`.
    ///
    /// It is the definition member's closure (which reaches only the part arguments the
    /// member reads) plus the instance path; for an interface-typed `slot` it also folds in
    /// the slot part's own digest, so a swap stales every instance of that slot only.
    pub fn instance_closure(
        &self,
        member: &str,
        instance: &str,
        interface_slot: Option<&str>,
    ) -> Result<Digest, DigestError> {
        let base = self.closure(member)?;
        let slot_own = interface_slot.map(|s| self.own(s)).transpose()?;
        let mut e = Enc::new("instance");
        e.digest(&base).str(instance);
        e.opt(&slot_own, |e, d| {
            e.digest(d);
        });
        Ok(e.finish())
    }
}

// frob:ticket 01M4VQXPDA4TEKE455FNYB5TXC
/// The graph dependency of a frame path: none for a component origin (not a node).
fn frame_dep(p: &crate::path::Path) -> Vec<String> {
    let t = p.to_string();
    if t.ends_with(".origin") {
        Vec::new()
    } else {
        vec![t]
    }
}

/// The argument member paths of an op chain, as text.
fn op_paths(chain: &[crate::geometry::Op]) -> Vec<String> {
    use crate::geometry::Op;
    let mut out = Vec::new();
    for o in chain {
        match o {
            Op::Offset(a) => out.extend(
                [&a.x, &a.y, &a.z]
                    .into_iter()
                    .flatten()
                    .map(ToString::to_string),
            ),
            Op::Rotate { angle, .. } => out.push(angle.to_string()),
            Op::Flip { .. } => {}
        }
    }
    out
}

/// Fill every digest and closure in `project`; returns an error on cycles or unknown deps.
///
/// Part slots are keyed by `Part.path`. Closures are set on members only (components,
/// checks and solvers carry own digests).
pub fn digest_project(project: &mut Project) -> Result<(), DigestError> {
    let mut g = DigestGraph::new();
    for c in &project.components {
        // frob:ticket 01M4W8SZ3HZZ2G8V1QVC7DZBZH
        // A component argument is an input node: members that read it depend on it.
        g.add_params(c);
        for m in &c.members {
            g.add_member(m);
        }
        for p in &c.parts {
            g.add_part(&p.path.to_string(), p);
        }
        for f in &c.fits {
            g.add_fit(f);
        }
        // frob:ticket 01M4VQXPDA4TEKE455FNYB5TXC
        for d in &c.datums {
            g.add_datum(d);
        }
        for p in &c.placements {
            g.add_placement(p);
        }
    }
    for s in &project.solvers {
        g.add_solver(s);
    }
    let closures = g.closures()?;
    for c in &mut project.components {
        for m in &mut c.members {
            let k = m.path.to_string();
            m.digest = Some(g.own(&k)?.clone());
            m.closure = closures.get(&k).cloned();
        }
        // frob:ticket 01M4VQXPDA4TEKE455FNYB5TXC
        for d in &mut c.datums {
            let k = d.path.to_string();
            d.digest = Some(g.own(&k)?.clone());
            d.closure = closures.get(&k).cloned();
        }
        for p in &mut c.placements {
            let k = p.path.to_string();
            p.digest = Some(g.own(&k)?.clone());
            p.closure = closures.get(&k).cloned();
        }
        c.digest = Some(component_digest(c));
    }
    project.digest = Some(project_digest(project));
    debug!("digest: project digested");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        AffineTerm, Citation, ComponentClass, Flavour, MemberKind, PackageId, SpanRef,
    };
    use crate::path::{Path, PathContext};
    use cadetta_units::{Dim, Magnitude, Rational};

    fn p(s: &str) -> Path {
        Path::parse(s, PathContext::General).unwrap()
    }

    fn item(s: &str) -> Path {
        let mut path = p(&format!("{s}.x"));
        path.members.clear();
        path
    }

    fn ex(n: i64) -> Exact {
        Exact(Magnitude::new(Rational::new(n, 1).unwrap(), 0))
    }

    fn mem(path: &str, n: i64, deps: &[&str]) -> Member {
        let mut m = Member::new(p(path), MemberKind::Param);
        m.value = Some(Value::new(
            Nominal::Exact(ex(n)),
            Dim::default(),
            n.to_string(),
        ));
        m.deps = deps.iter().map(|d| p(d)).collect();
        m
    }

    fn part(args: &[(&str, &str)], primary: &str) -> Part {
        Part {
            path: p("a.b::Rover.wheel"),
            type_name: "Wheel".into(),
            count: Some(4),
            primary: primary.into(),
            alternates: vec!["Alt".into()],
            args: args
                .iter()
                .map(|(k, v)| ((*k).into(), (*v).into()))
                .collect(),
        }
    }

    // frob:tests digest_excludes_spans_docs_formatting
    #[test]
    fn digest_excludes_spans_docs_formatting() {
        let a = mem("a.b::X.d", 3, &[]);
        let mut b = a.clone();
        b.doc = Some("docs".into());
        b.span = Some(SpanRef {
            file: "f".into(),
            start: 1,
            end: 2,
        });
        b.value.as_mut().unwrap().authored = "3.000".into();
        assert_eq!(member_digest(&a), member_digest(&b));
        let mut ga = DigestGraph::new();
        let mut gb = DigestGraph::new();
        ga.add_member(&a);
        gb.add_member(&b);
        assert_eq!(ga.closure("a.b::X.d"), gb.closure("a.b::X.d"));
    }

    // frob:tests digest_changed_value_changes_own
    #[test]
    fn digest_changed_value_changes_own() {
        assert_ne!(
            member_digest(&mem("a.b::X.d", 3, &[])),
            member_digest(&mem("a.b::X.d", 4, &[]))
        );
        let mut t = mem("a.b::X.d", 3, &[]);
        t.value.as_mut().unwrap().tol = Some(Affine {
            terms: vec![AffineTerm {
                var: p("a.b::X.d"),
                lo: ex(-1),
                hi: ex(1),
            }],
        });
        assert_ne!(member_digest(&t), member_digest(&mem("a.b::X.d", 3, &[])));
    }

    // frob:tests digest_closure_merkle_over_deps
    #[test]
    fn digest_closure_merkle_over_deps() {
        let build = |v: i64, order: bool| {
            let mut g = DigestGraph::new();
            g.add_member(&mem("a.b::X.a", v, &[]));
            g.add_member(&mem("a.b::X.b", 2, &[]));
            let deps = if order {
                ["a.b::X.a", "a.b::X.b"]
            } else {
                ["a.b::X.b", "a.b::X.a"]
            };
            g.add_member(&mem("a.b::X.c", 9, &deps));
            g
        };
        let g = build(1, true);
        assert_eq!(g.closure("a.b::X.c"), build(1, false).closure("a.b::X.c"));
        assert_ne!(g.closure("a.b::X.c"), build(5, true).closure("a.b::X.c"));
        assert_ne!(g.closure("a.b::X.c").unwrap(), *g.own("a.b::X.c").unwrap());
        // a citing node's digest covers its CiteRef only, never the cited item's digest
        let mut m = mem("a.b::X.f", 1, &[]);
        let before = member_digest(&m);
        m.citation = Some(CiteRef::item("a.b::refs::n", Some("p.2".into())));
        let c1 = member_digest(&m);
        assert_ne!(before, c1);
        m.citation = Some(CiteRef::item("a.b::refs::n", Some("p.3".into())));
        assert_ne!(c1, member_digest(&m));
        let mut s = Solver {
            path: p("a.b::X.sol"),
            script: "s.py".into(),
            citation: None,
            inputs: BTreeMap::new(),
            outputs: BTreeMap::new(),
            deps: vec![],
            result_digest: Some(Digest("11".into())),
            stale: false,
        };
        let reader = mem("a.b::X.r", 1, &["a.b::X.sol"]);
        let close = |s: &Solver| {
            let mut g = DigestGraph::new();
            g.add_solver(s);
            g.add_member(&reader);
            g.closure("a.b::X.r").unwrap()
        };
        let c = close(&s);
        s.result_digest = Some(Digest("22".into()));
        assert_ne!(c, close(&s));
        // the method citation is documentation, never an input to the solver closure (solvers.md 1)
        s.citation = Some(CiteRef::item("a.b::refs::method", None));
        let c2 = close(&s);
        s.citation = None;
        s.result_digest = Some(Digest("22".into()));
        assert_eq!(c2, close(&s));
    }

    // frob:tests digest_graph_errors
    #[test]
    fn digest_graph_errors() {
        let mut g = DigestGraph::new();
        g.add_member(&mem("a.b::X.a", 1, &["a.b::X.b"]));
        assert!(matches!(
            g.closure("a.b::X.a"),
            Err(DigestError::MissingDep { .. })
        ));
        g.add_member(&mem("a.b::X.b", 1, &["a.b::X.a"]));
        assert!(matches!(g.closure("a.b::X.a"), Err(DigestError::Cycle(_))));
        assert!(matches!(g.closure("nope"), Err(DigestError::Unknown(_))));
    }

    fn rover(primary: &str, w_expr: &str, ifc: bool) -> (DigestGraph, Digest, Digest, Digest) {
        let slot = "a.b::Rover.wheel";
        let mut g = DigestGraph::new();
        g.add_part(slot, &part(&[("d", "100 mm"), ("w", w_expr)], primary));
        g.add_member(&mem("a.b::Wheel.hub_bore", 1, &["a.b::Rover.wheel.d"]));
        g.add_member(&mem("a.b::Rover.chk", 1, &["a.b::Rover.wheel.w"]));
        let iface = ifc.then_some(slot);
        let hub = g
            .instance_closure("a.b::Wheel.hub_bore", "Rover/wheel[2].hub_bore", iface)
            .unwrap();
        let hub1 = g
            .instance_closure("a.b::Wheel.hub_bore", "Rover/wheel[1].hub_bore", iface)
            .unwrap();
        (g.clone(), hub, hub1, g.closure("a.b::Rover.chk").unwrap())
    }

    // frob:tests digest_instance_closure_reads_only_used_args
    #[test]
    fn digest_instance_closure_reads_only_used_args() {
        let (_, hub, hub1, chk) = rover("A", "10 mm", false);
        let (_, hub_w, _, chk_w) = rover("A", "20 mm", false);
        assert_eq!(hub, hub_w, "unread arg w must not change hub_bore");
        assert_ne!(chk, chk_w);
        assert_ne!(hub, hub1);
    }

    // frob:tests digest_part_includes_primary_alternates_args
    #[test]
    fn digest_part_includes_primary_alternates_args() {
        let base = part(&[("d", "1")], "A");
        assert_ne!(part_digest(&base), part_digest(&part(&[("d", "1")], "B")));
        assert_ne!(part_digest(&base), part_digest(&part(&[("d", "2")], "A")));
        let mut alt = base.clone();
        alt.alternates = vec!["Other".into()];
        assert_ne!(part_digest(&base), part_digest(&alt));
    }

    // frob:tests digest_interface_slot_swap
    #[test]
    fn digest_interface_slot_swap() {
        let (_, hub_a, hub1_a, chk_a) = rover("A", "10 mm", true);
        let (_, hub_b, hub1_b, chk_b) = rover("B", "10 mm", true);
        assert_ne!(hub_a, hub_b);
        assert_ne!(hub1_a, hub1_b);
        assert_eq!(chk_a, chk_b);
    }

    // frob:tests digest_semantic_attrs
    #[test]
    fn digest_semantic_attrs() {
        let base = mem("a.b::X.d", 3, &[]);
        for k in PRESENTATIONAL_ATTRS {
            let mut m = base.clone();
            m.attrs.insert(k.into(), "v".into());
            assert_eq!(member_digest(&base), member_digest(&m), "{k}");
        }
        for k in ["process", "default", "hole"] {
            let mut m = base.clone();
            m.attrs.insert(k.into(), "v".into());
            assert_ne!(member_digest(&base), member_digest(&m), "{k}");
        }
        let mut r = base.clone();
        r.role = Some("hole".into());
        assert_ne!(member_digest(&base), member_digest(&r));
    }

    // frob:tests digest_moved_datum
    #[test]
    fn digest_moved_datum() {
        let datum = |n| {
            let mut m = mem("a.b::Rover.mount.offset.x", n, &[]);
            m.kind = MemberKind::Datum;
            m
        };
        let build = |n| {
            let mut g = DigestGraph::new();
            g.add_member(&datum(n));
            g.add_member(&mem("a.b::Rover.reader", 1, &["a.b::Rover.mount.offset.x"]));
            g.add_member(&mem("a.b::Rover.other", 1, &[]));
            g
        };
        let (a, b) = (build(1), build(2));
        assert_ne!(
            a.closure("a.b::Rover.reader"),
            b.closure("a.b::Rover.reader")
        );
        assert_eq!(a.closure("a.b::Rover.other"), b.closure("a.b::Rover.other"));
    }

    // frob:tests DigestGraph.add_params
    #[test]
    fn member_reading_a_component_argument_digests() {
        let mut pr = Project::new(
            Flavour::Design,
            PackageId {
                name: "a.b".into(),
                version: "1".into(),
            },
        );
        pr.components.push(Component {
            path: item("a.b::X"),
            class: ComponentClass::Model,
            params_sig: Some("motor, w".into()),
            implements: vec![],
            citation: None,
            attrs: BTreeMap::new(),
            members: vec![mem("a.b::X.a", 1, &["a.b::X.motor"])],
            parts: vec![],
            fits: vec![],
            datums: vec![],
            points: vec![],
            placements: vec![],
            span: None,
            digest: None,
        });
        digest_project(&mut pr).unwrap();
        assert!(pr.components[0].members[0].closure.is_some());
    }

    // frob:tests digest_project_fills_all
    #[test]
    fn digest_project_fills_all() {
        let mut pr = Project::new(
            Flavour::Design,
            PackageId {
                name: "a.b".into(),
                version: "1".into(),
            },
        );
        pr.components.push(Component {
            path: p("a.b::X.d"),
            class: ComponentClass::Model,
            params_sig: None,
            implements: vec![],
            citation: None,
            attrs: BTreeMap::new(),
            members: vec![mem("a.b::X.a", 1, &[]), mem("a.b::X.b", 2, &["a.b::X.a"])],
            parts: vec![],
            fits: vec![],
            datums: vec![],
            points: vec![],
            placements: vec![],
            span: None,
            digest: None,
        });
        digest_project(&mut pr).unwrap();
        let first = pr.digest.clone().unwrap();
        assert!(
            pr.components[0]
                .members
                .iter()
                .all(|m| m.digest.is_some() && m.closure.is_some())
        );
        assert!(pr.components[0].digest.is_some());
        pr.components[0].members[0].value.as_mut().unwrap().nominal = Nominal::Exact(ex(7));
        digest_project(&mut pr).unwrap();
        assert_ne!(first, pr.digest.clone().unwrap());
        assert_eq!(digest_bytes(b"x").0.len(), 64);
    }

    // frob:ticket 01M4VB62VJQV9AKSSQME8NWW6Y
    // frob:tests crates/cadetta-ir/src/digest.rs::citation_digest kind=unit
    #[test]
    fn citation_digest_covers_fields_and_file_not_citers() {
        let fields =
            |t: &str| BTreeMap::from([("title".to_owned(), CiteField::Text(t.to_owned()))]);
        let base = Citation::new("a.b::refs::n", CitationType::Web, fields("T"), None);
        let edited = Citation::new("a.b::refs::n", CitationType::Web, fields("U"), None);
        assert_ne!(base.digest, edited.digest);
        let with_file = Citation::new(
            "a.b::refs::n",
            CitationType::Web,
            fields("T"),
            Some(Digest("aa".into())),
        );
        assert_ne!(
            base.digest, with_file.digest,
            "absent marker differs from a digest"
        );
        let changed_file = Citation::new(
            "a.b::refs::n",
            CitationType::Web,
            fields("T"),
            Some(Digest("bb".into())),
        );
        assert_ne!(with_file.digest, changed_file.digest);
        // the citing member and its closure do not move when the citation is edited
        let mut m = mem("a.b::X.f", 1, &[]);
        m.citation = Some(CiteRef::item("a.b::refs::n", None));
        let mut g = DigestGraph::new();
        g.add_member(&m);
        let (own, closure) = (member_digest(&m), g.closure("a.b::X.f").unwrap());
        let mut pr = Project::new(
            Flavour::Design,
            PackageId {
                name: "a.b".into(),
                version: "0.1.0".into(),
            },
        );
        pr.citations = vec![base];
        let before = project_digest(&pr);
        pr.citations = vec![edited];
        assert_ne!(
            before,
            project_digest(&pr),
            "the project digest sees the edit"
        );
        assert_eq!(own, member_digest(&m));
        assert_eq!(closure, g.closure("a.b::X.f").unwrap());
    }
}
