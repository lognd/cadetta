//! Dependency graph, reference cycles (D003) and `was` renames (N004).
//!
//! The graph is built from the syntax tree and the [`Resolver`] alone; no
//! value is evaluated (language.md intro, 3.5). Nodes are *members of
//! instances*, not components: an instance is a root component plus the chain
//! of `part` names that reach it, and a node is one parameter, `let`, check,
//! `part` slot or component argument of such an instance. Edges are the
//! references a node's definition reads. Cycles are strongly connected
//! components of this graph, so a model argument that comes from a joint
//! reading another member of the same model is not a cycle (language.md 4).
//!
//! Decisions made here (written into language.md 3.5 and 4): reading a member
//! of an array `part` also depends on the part slot, because the instances
//! exist only once the count is known; a roll-up (`sum(self, mass)`,
//! `mass(self)`) depends on the slot and the named member of every part of
//! the component; a `was` path is read syntactically (the old item may no
//! longer exist) and a reference to a name that no longer resolves is
//! redirected through it with N004.

// frob:ticket 01M4SPWPG1WBAHFPZ19ZV3BAPJ

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use cadetta_syntax::ast::{
    Arg, Block, Component, Expr, ExprKind, MemberKind, Path, PathSep, Type, TypeKind,
};
use cadetta_syntax::span::Span;
use tracing::{debug, trace, warn};

use crate::resolve::{
    CanonPath, Ctx, Index, ItemClass, Level, MemberSeg, ModuleKey, PackageSrc, PartSeg,
    ResolveError, Resolved, Resolver,
};

/// Members every part contributes to a `name(self)` roll-up (language.md 3.5).
const ROLLUP_NAMES: &[&str] = &["unit_cost", "lead_time", "moq", "supplier", "mass"];

/// Roll-up functions that take `(self, member)`.
const ROLLUP_FNS: &[&str] = &["sum", "max", "min"];

/// How serious a dependency diagnostic is.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Severity {
    /// A fixed-severity error (D003).
    Error,
    /// A warning (N004, N006).
    Warning,
}

/// A D003, N004 or N006 finding.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct DepDiagnostic {
    /// The lint code.
    pub code: &'static str,
    /// Error or warning.
    pub severity: Severity,
    /// The human message.
    pub message: String,
    /// The primary span.
    pub span: Span,
    /// Other spans taking part (the rest of a cycle).
    pub related: Vec<Span>,
    /// The module the primary span lies in, when known.
    pub module: Option<ModuleKey>,
}

impl fmt::Display for DepDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

/// Maps the old canonical path of each `was` to the parameter's current path.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Renames {
    map: BTreeMap<CanonPath, CanonPath>,
}

/// One node of the graph: a member of an instance and the nodes it reads.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Node {
    /// Where the member is declared.
    pub span: Span,
    /// The nodes this member's definition reads.
    pub edges: BTreeSet<CanonPath>,
}

/// The member-level dependency graph of a project, with its diagnostics.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct DepGraph {
    nodes: BTreeMap<CanonPath, Node>,
    diagnostics: Vec<DepDiagnostic>,
}

/// An instance: the root component and the `part` names leading to it.
#[derive(Clone, Debug)]
struct Inst {
    root: CanonPath,
    parts: Vec<String>,
}

impl Inst {
    fn key(&self, member: &str) -> CanonPath {
        node_key(&self.root, &self.parts, member)
    }
}

fn node_key(root: &CanonPath, parts: &[String], member: &str) -> CanonPath {
    CanonPath {
        parts: parts
            .iter()
            .map(|p| PartSeg {
                name: p.clone(),
                index: None,
            })
            .collect(),
        members: vec![MemberSeg::Name(member.to_owned())],
        ..root.item_only()
    }
}

/// The site a `part` was declared at, supplying its component's arguments.
struct Site<'a> {
    comp_id: CanonPath,
    inst: Inst,
    ctx: Ctx,
    args: &'a [Arg],
}

impl Renames {
    /// Collects every `was` in `packages` as old path -> current path.
    pub fn collect(resolver: &Resolver<'_>, packages: &[PackageSrc<'_>]) -> Self {
        let mut map = BTreeMap::new();
        for p in packages {
            for m in &p.modules {
                for c in m.file.components() {
                    let cid = CanonPath::item(&p.name, &m.path, &c.name.name);
                    let ctx = Ctx {
                        package: p.name.clone(),
                        module: m.path.clone(),
                        component: Some(c.name.name.clone()),
                        locals: Vec::new(),
                    };
                    for mem in &c.members {
                        let MemberKind::Param(param) = &mem.kind else {
                            continue;
                        };
                        for was in &param.was {
                            let Some(old) = path_to_canon(resolver, &ctx, was) else {
                                warn!(path = %was.text(), "unreadable `was` path");
                                continue;
                            };
                            let mut new = cid.clone();
                            new.members = vec![MemberSeg::Name(param.name.name.clone())];
                            debug!(%old, %new, "rename recorded");
                            map.insert(old, new);
                        }
                    }
                }
            }
        }
        Self { map }
    }

    /// Resolves like [`Resolver::resolve_ref`]; a reference that no longer
    /// resolves but names a `was` path resolves to the current path with N004.
    pub fn resolve_ref(
        &self,
        resolver: &Resolver<'_>,
        ctx: &Ctx,
        expr: &Expr,
        expected: Option<&Type>,
    ) -> Result<(Resolved, Option<DepDiagnostic>), ResolveError> {
        match resolver.resolve_ref(ctx, expr, expected) {
            Ok(r) => Ok((r, None)),
            Err(e) if e.code == "E002" => match self.redirect(resolver, ctx, expr) {
                Some((new, old)) => {
                    debug!(%old, %new, "reference redirected through `was`");
                    Ok((
                        Resolved::Member {
                            path: new.clone(),
                            level: Level::Definition,
                            beside_instance: None,
                        },
                        Some(DepDiagnostic {
                            module: Some(ModuleKey::of_ctx(ctx)),
                            code: "N004",
                            severity: Severity::Warning,
                            message: format!("`{old}` was renamed; use `{new}`"),
                            span: expr.span,
                            related: Vec::new(),
                        }),
                    ))
                }
                None => Err(e),
            },
            Err(e) => Err(e),
        }
    }

    fn redirect(
        &self,
        resolver: &Resolver<'_>,
        ctx: &Ctx,
        expr: &Expr,
    ) -> Option<(CanonPath, CanonPath)> {
        let (path, extra) = chain(expr)?;
        let mut old = path_to_canon(resolver, ctx, &path)?;
        old.members.extend(extra);
        // The longest recorded old path that prefixes the reference.
        let (key, new) = self.map.iter().find(|(k, _)| {
            k.item_only() == old.item_only()
                && k.parts == old.parts
                && old.members.starts_with(&k.members)
        })?;
        let mut redirected = new.clone();
        redirected
            .members
            .extend(old.members[key.members.len()..].iter().cloned());
        Some((redirected, old))
    }

    /// Every recorded rename as (old path, current path), in path order.
    // frob:ticket 01M4SXS3P4EGP92DE0KZ81R7HK
    pub fn entries(&self) -> impl Iterator<Item = (&CanonPath, &CanonPath)> {
        self.map.iter()
    }

    /// Number of recorded renames.
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// True when no `was` is recorded.
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

/// A source path as `Path` + trailing `.field` / literal `[index]` members, if it is a pure chain.
fn chain(e: &Expr) -> Option<(Path, Vec<MemberSeg>)> {
    match &e.kind {
        ExprKind::Path(p) => Some((p.clone(), Vec::new())),
        ExprKind::Paren(x) => chain(x),
        ExprKind::Field(x, id) => {
            let (p, mut m) = chain(x)?;
            m.push(MemberSeg::Name(id.name.clone()));
            Some((p, m))
        }
        ExprKind::Index(x, idx) => {
            let (p, mut m) = chain(x)?;
            let ExprKind::Number(t) = &idx.kind else {
                return None;
            };
            m.push(MemberSeg::Index(Index::At(t.parse().ok()?)));
            Some((p, m))
        }
        _ => None,
    }
}

fn is_pure_chain(e: &Expr) -> bool {
    chain(e).is_some()
}

/// The canonical form of a (possibly stale) path: the item prefix is resolved
/// when it exists and read textually otherwise; the rest are members.
fn path_to_canon(resolver: &Resolver<'_>, ctx: &Ctx, path: &Path) -> Option<CanonPath> {
    let item_idx = (1..path.segments.len())
        .rev()
        .find(|i| path.seps[i - 1] == PathSep::ColonColon);
    let sub = |end: usize| Path {
        segments: path.segments[..end].to_vec(),
        seps: path.seps[..end - 1].to_vec(),
        span: path.span,
    };
    let names = |from: usize| -> Vec<MemberSeg> {
        path.segments[from..]
            .iter()
            .map(|s| MemberSeg::Name(s.name.clone()))
            .collect()
    };
    if let Some(i) = item_idx {
        let prefix = sub(i + 1);
        let mut base = match resolver.resolve_path(ctx, &prefix, None) {
            Ok(Resolved::Item { path, .. }) => path,
            _ => textual_item(ctx, &prefix, i)?,
        };
        base.members = names(i + 1);
        return Some(base);
    }
    match resolver.resolve_path(ctx, &sub(1), None) {
        Ok(Resolved::Item { path, .. }) => {
            let mut base = path;
            base.members = names(1);
            Some(base)
        }
        Ok(_) => None,
        Err(_) if starts_upper(&path.segments[0].name) => {
            // A stale type name (`Old.thk`): read as an item of this module, like the `was`.
            // frob:ticket 01M4YR00ZFABJQWKQJ3F3JFVY4
            debug!(path = %path.text(), "stale item name read textually");
            let mut base = CanonPath::item(&ctx.package, &ctx.module, &path.segments[0].name);
            base.members = names(1);
            Some(base)
        }
        Err(_) => {
            let c = ctx.component.as_deref()?;
            let mut base = CanonPath::item(&ctx.package, &ctx.module, c);
            base.members = names(0);
            Some(base)
        }
    }
}

/// True when `name` starts with an uppercase letter (an item name).
fn starts_upper(name: &str) -> bool {
    name.chars().next().is_some_and(char::is_uppercase)
}

/// An item path read from text alone (its module or item has since moved).
fn textual_item(ctx: &Ctx, prefix: &Path, item_idx: usize) -> Option<CanonPath> {
    let seg = |i: usize| prefix.segments[i].name.as_str();
    let first_colon = (1..=item_idx).find(|i| prefix.seps[i - 1] == PathSep::ColonColon)?;
    let head = (0..first_colon).map(seg).collect::<Vec<_>>().join(".");
    let mods = (first_colon..item_idx)
        .map(|i| seg(i).to_owned())
        .collect::<Vec<_>>();
    let (package, modules) = match head.as_str() {
        "package" => (ctx.package.clone(), mods),
        "self" => (
            ctx.package.clone(),
            ctx.module.iter().cloned().chain(mods).collect(),
        ),
        "super" => {
            let up = ctx.module.split_last()?.1;
            (
                ctx.package.clone(),
                up.iter().cloned().chain(mods).collect(),
            )
        }
        _ => (head, mods),
    };
    Some(CanonPath::item(&package, &modules, seg(item_idx)))
}

impl DepGraph {
    /// Builds the graph of every component in `packages` without evaluating anything.
    pub fn build(resolver: &Resolver<'_>, renames: &Renames, packages: &[PackageSrc<'_>]) -> Self {
        let mut comps = BTreeMap::new();
        let mut case_diags = Vec::new();
        for p in packages {
            for m in &p.modules {
                for c in m.file.components() {
                    comps.insert(CanonPath::item(&p.name, &m.path, &c.name.name), c);
                    if !starts_upper(&c.name.name) {
                        // frob:ticket 01M4YR00ZFABJQWKQJ3F3JFVY4
                        // Lowering drops such a component, so the IR lint never sees the name.
                        debug!(name = %c.name.name, "N001: component name is not an item name");
                        case_diags.push(DepDiagnostic {
                            module: Some(ModuleKey::new(&p.name, &m.path)),
                            code: "N001",
                            severity: Severity::Warning,
                            message: format!(
                                "component name `{}` breaks the case canon",
                                c.name.name
                            ),
                            span: c.name.span,
                            related: Vec::new(),
                        });
                    }
                }
            }
        }
        let mut b = Builder {
            resolver,
            renames,
            comps,
            graph: DepGraph::default(),
        };
        let ids = b.comps.keys().cloned().collect::<Vec<_>>();
        for id in &ids {
            let inst = Inst {
                root: id.clone(),
                parts: Vec::new(),
            };
            b.add_instance(id, &inst, None, &mut Vec::new());
        }
        let mut graph = b.graph;
        graph.diagnostics.extend(case_diags);
        graph.diagnostics.extend(graph.find_cycles());
        graph.diagnostics.sort_by_key(|d| (d.span, d.code));
        graph.diagnostics.dedup();
        debug!(
            nodes = graph.nodes.len(),
            diagnostics = graph.diagnostics.len(),
            "dependency graph built"
        );
        graph
    }

    /// Every node, keyed by instance-level member path.
    pub fn nodes(&self) -> &BTreeMap<CanonPath, Node> {
        &self.nodes
    }

    /// The D003 and N004 diagnostics, ordered by span.
    pub fn diagnostics(&self) -> &[DepDiagnostic] {
        &self.diagnostics
    }

    /// The nodes `from` reads, or none when `from` is not a node.
    pub fn deps_of(&self, from: &CanonPath) -> impl Iterator<Item = &CanonPath> {
        self.nodes
            .get(from)
            .into_iter()
            .flat_map(|n| n.edges.iter())
    }

    /// One D003 per distinct cycle (strongly connected component or self-loop).
    fn find_cycles(&self) -> Vec<DepDiagnostic> {
        let mut out = Vec::new();
        let mut seen: BTreeSet<Vec<Span>> = BTreeSet::new();
        for scc in Tarjan::run(&self.nodes) {
            let looped = scc.len() == 1 && self.nodes[&scc[0]].edges.contains(&scc[0]);
            if scc.len() < 2 && !looped {
                continue;
            }
            let mut members = scc
                .iter()
                .map(|k| (self.nodes[k].span, k.clone()))
                .collect::<Vec<_>>();
            members.sort();
            let spans = members.iter().map(|(s, _)| *s).collect::<Vec<_>>();
            if !seen.insert(spans.clone()) {
                continue;
            }
            let names = members
                .iter()
                .map(|(_, k)| k.to_string())
                .chain(std::iter::once(members[0].1.to_string()))
                .collect::<Vec<_>>()
                .join(" -> ");
            warn!(cycle = %names, "reference cycle");
            out.push(DepDiagnostic {
                module: Some(ModuleKey::of_path(&members[0].1)),
                code: "D003",
                severity: Severity::Error,
                message: format!("reference cycle: {names}"),
                span: spans[0],
                related: spans[1..].to_vec(),
            });
        }
        out
    }
}

/// Tarjan's strongly connected components over the node map.
struct Tarjan<'g> {
    nodes: &'g BTreeMap<CanonPath, Node>,
    next: usize,
    index: BTreeMap<&'g CanonPath, usize>,
    low: BTreeMap<&'g CanonPath, usize>,
    stack: Vec<&'g CanonPath>,
    on: BTreeSet<&'g CanonPath>,
    out: Vec<Vec<CanonPath>>,
}

impl<'g> Tarjan<'g> {
    fn run(nodes: &'g BTreeMap<CanonPath, Node>) -> Vec<Vec<CanonPath>> {
        let mut t = Self {
            nodes,
            next: 0,
            index: BTreeMap::new(),
            low: BTreeMap::new(),
            stack: Vec::new(),
            on: BTreeSet::new(),
            out: Vec::new(),
        };
        for k in nodes.keys() {
            if !t.index.contains_key(k) {
                t.visit(k);
            }
        }
        t.out
    }

    fn visit(&mut self, v: &'g CanonPath) {
        self.index.insert(v, self.next);
        self.low.insert(v, self.next);
        self.next += 1;
        self.stack.push(v);
        self.on.insert(v);
        for (w, _) in self.nodes[v]
            .edges
            .iter()
            .filter_map(|w| self.nodes.get_key_value(w))
        {
            if !self.index.contains_key(w) {
                self.visit(w);
                let lw = self.low[w];
                let lv = self.low[v].min(lw);
                self.low.insert(v, lv);
            } else if self.on.contains(w) {
                let iw = self.index[w];
                let lv = self.low[v].min(iw);
                self.low.insert(v, lv);
            }
        }
        if self.low[v] == self.index[v] {
            let mut scc = Vec::new();
            while let Some(w) = self.stack.pop() {
                self.on.remove(w);
                scc.push(w.clone());
                if std::ptr::eq(w, v) {
                    break;
                }
            }
            self.out.push(scc);
        }
    }
}

struct Builder<'r, 'a> {
    resolver: &'r Resolver<'r>,
    renames: &'r Renames,
    comps: BTreeMap<CanonPath, &'a Component>,
    graph: DepGraph,
}

impl<'a> Builder<'_, 'a> {
    /// Adds the nodes of one instance of `comp_id`, then of the instances of its parts.
    fn add_instance(
        &mut self,
        comp_id: &CanonPath,
        inst: &Inst,
        parent: Option<&Site<'_>>,
        stack: &mut Vec<CanonPath>,
    ) {
        if stack.contains(comp_id) {
            warn!(%comp_id, "recursive part type; not expanded");
            return;
        }
        let Some(comp) = self.comps.get(comp_id).copied() else {
            return;
        };
        stack.push(comp_id.clone());
        let ctx = Ctx {
            package: comp_id.package.clone(),
            module: comp_id.modules.clone(),
            component: Some(comp.name.name.clone()),
            locals: Vec::new(),
        };
        let own = Site {
            comp_id: comp_id.clone(),
            inst: inst.clone(),
            ctx,
            args: &[],
        };
        for (i, p) in comp.params.iter().enumerate() {
            let supplied = parent.and_then(|s| {
                s.args
                    .iter()
                    .enumerate()
                    .find(|(j, a)| a.name.as_ref().map_or(*j == i, |n| n.name == p.name.name))
                    .map(|(_, a)| (s, &a.value))
            });
            let mut edges = BTreeSet::new();
            match (supplied, &p.default) {
                // frob:ticket 01M4TRBJ7D2JTGX6XX94BJ1ZGM
                (Some((site, value)), _) => {
                    let bare = matches!(
                        self.resolver
                            .resolve_ref_in_param_type(&site.ctx, value, &p.ty, comp_id),
                        Ok(Resolved::Variant { .. })
                    );
                    if !bare {
                        self.reads(value, &[], site, comp, &mut edges);
                    }
                }
                (None, Some(d)) => self.reads(d, &[], &own, comp, &mut edges),
                (None, None) => {}
            }
            self.graph.insert(inst.key(&p.name.name), p.span, edges);
        }
        for m in &comp.members {
            self.add_member(m, comp, &own, stack);
        }
        stack.pop();
    }

    fn add_member(
        &mut self,
        m: &cadetta_syntax::ast::Member,
        comp: &'a Component,
        own: &Site<'_>,
        stack: &mut Vec<CanonPath>,
    ) {
        let inst = &own.inst;
        let mut edges = BTreeSet::new();
        let name = match &m.kind {
            MemberKind::Param(p) => {
                self.reads(&p.value, &[], own, comp, &mut edges);
                &p.name.name
            }
            MemberKind::Let(l) => {
                self.reads(&l.value, &[], own, comp, &mut edges);
                &l.name.name
            }
            MemberKind::Check(c) => {
                let mut bound = Vec::new();
                if let Some((v, e)) = &c.each {
                    self.reads(e, &[], own, comp, &mut edges);
                    bound.push(v.name.clone());
                }
                self.reads(&c.cond, &bound, own, comp, &mut edges);
                &c.name.name
            }
            MemberKind::Require(r) => {
                self.reads(&r.cond, &[], own, comp, &mut edges);
                &r.name.name
            }
            MemberKind::Fit(f) => {
                let on = Expr {
                    kind: ExprKind::Path(f.on.clone()),
                    span: f.on.span,
                };
                self.reads(&on, &[], own, comp, &mut edges);
                if let Some((lo, hi)) = &f.range {
                    self.reads(lo, &[], own, comp, &mut edges);
                    self.reads(hi, &[], own, comp, &mut edges);
                }
                &f.name.name
            }
            MemberKind::Datum(d) => {
                if let Some(v) = &d.value {
                    self.reads(v, &[], own, comp, &mut edges);
                }
                &d.name.name
            }
            MemberKind::Part(p) => {
                if let Some(count) = &p.count {
                    self.reads(count, &[], own, comp, &mut edges);
                }
                self.graph.insert(inst.key(&p.name.name), m.span, edges);
                self.add_part(p, own, stack);
                return;
            }
            _ => return,
        };
        self.graph.insert(inst.key(name), m.span, edges);
    }

    /// Expands the instance a `part` declares, wiring its arguments to the declaring site.
    fn add_part(
        &mut self,
        p: &cadetta_syntax::ast::Part,
        own: &Site<'_>,
        stack: &mut Vec<CanonPath>,
    ) {
        let (ty_path, args) = match &p.primary {
            Some(c) => (&c.path, c.args.as_deref()),
            None => match &p.ty.kind {
                TypeKind::Path(tp) => (tp, p.args.as_deref()),
                _ => return,
            },
        };
        let scope = Ctx {
            component: None,
            ..own.ctx.clone()
        };
        let Ok(Resolved::Item {
            path: child,
            class: ItemClass::Component,
        }) = self.resolver.resolve_path(&scope, ty_path, None)
        else {
            trace!(part = %p.name.name, "part type is not a component; not expanded");
            return;
        };
        let mut parts = own.inst.parts.clone();
        parts.push(p.name.name.clone());
        let child_inst = Inst {
            root: own.inst.root.clone(),
            parts,
        };
        let site = Site {
            comp_id: own.comp_id.clone(),
            inst: own.inst.clone(),
            ctx: own.ctx.clone(),
            args: args.unwrap_or(&[]),
        };
        self.add_instance(&child, &child_inst, Some(&site), stack);
    }

    /// Collects into `out` the nodes `e` reads, resolved in `site`.
    fn reads(
        &mut self,
        e: &Expr,
        bound: &[String],
        site: &Site<'_>,
        comp: &Component,
        out: &mut BTreeSet<CanonPath>,
    ) {
        let mut bound = bound.to_vec();
        self.walk(e, &mut bound, site, comp, out);
    }

    fn walk(
        &mut self,
        e: &Expr,
        bound: &mut Vec<String>,
        site: &Site<'_>,
        comp: &Component,
        out: &mut BTreeSet<CanonPath>,
    ) {
        macro_rules! go {
            ($x:expr) => {
                self.walk($x, bound, site, comp, out)
            };
        }
        match &e.kind {
            ExprKind::Number(_) | ExprKind::Quantity(_) | ExprKind::Str(_) | ExprKind::Bool(_) => {}
            ExprKind::Path(_) => self.reference(e, bound, site, comp, out),
            ExprKind::Field(x, _) | ExprKind::Index(x, _) if is_pure_chain(e) => {
                let _ = x;
                self.reference(e, bound, site, comp, out);
            }
            ExprKind::Field(x, _) => go!(x),
            ExprKind::Index(x, i) => {
                go!(x);
                go!(i);
            }
            ExprKind::Paren(x)
            | ExprKind::Unary(_, x)
            | ExprKind::As(x, _)
            | ExprKind::IsoClass(x, _) => go!(x),
            ExprKind::Array(xs) => xs.iter().for_each(|x| go!(x)),
            ExprKind::Binary(_, a, b) | ExprKind::Satisfies(a, b) => {
                go!(a);
                go!(b);
            }
            ExprKind::Compare { first, rest } => {
                go!(first);
                rest.iter().for_each(|(_, x)| go!(x));
            }
            ExprKind::Call { callee, args, .. } => {
                if let Some(member) = rollup_member(callee, args) {
                    self.rollup(&member, site, comp, out);
                    return;
                }
                if !matches!(callee.kind, ExprKind::Path(_)) {
                    go!(callee);
                }
                args.iter().for_each(|a| go!(&a.value));
            }
            ExprKind::Tolerance(x, terms) => {
                go!(x);
                terms.iter().for_each(|t| go!(&t.value));
            }
            ExprKind::If {
                cond,
                then,
                otherwise,
            } => {
                go!(cond);
                self.block(then, bound, site, comp, out);
                if let Some(o) = otherwise {
                    go!(o);
                }
            }
            ExprKind::Match { scrutinee, arms } => {
                go!(scrutinee);
                arms.iter().for_each(|a| go!(&a.body));
            }
            ExprKind::Block(b) => self.block(b, bound, site, comp, out),
        }
    }

    fn block(
        &mut self,
        b: &Block,
        bound: &mut Vec<String>,
        site: &Site<'_>,
        comp: &Component,
        out: &mut BTreeSet<CanonPath>,
    ) {
        let mark = bound.len();
        for l in &b.lets {
            self.walk(&l.value, bound, site, comp, out);
            bound.push(l.name.name.clone());
        }
        self.walk(&b.tail, bound, site, comp, out);
        bound.truncate(mark);
    }

    /// A roll-up depends on every part slot and on `member` of every part.
    fn rollup(
        &mut self,
        member: &str,
        site: &Site<'_>,
        comp: &Component,
        out: &mut BTreeSet<CanonPath>,
    ) {
        for m in &comp.members {
            if let MemberKind::Part(p) = &m.kind {
                out.insert(site.inst.key(&p.name.name));
                let mut parts = site.inst.parts.clone();
                parts.push(p.name.name.clone());
                out.insert(node_key(&site.inst.root, &parts, member));
            }
        }
    }

    fn reference(
        &mut self,
        e: &Expr,
        bound: &[String],
        site: &Site<'_>,
        comp: &Component,
        out: &mut BTreeSet<CanonPath>,
    ) {
        let ctx = Ctx {
            locals: bound.to_vec(),
            ..site.ctx.clone()
        };
        let resolved = match self.renames.resolve_ref(self.resolver, &ctx, e, None) {
            Ok((r, diag)) => {
                self.graph.diagnostics.extend(diag);
                r
            }
            Err(err) => {
                trace!(%err, "unresolved reference skipped by the dependency graph");
                return;
            }
        };
        if let Resolved::Member {
            path,
            beside_instance: Some(part),
            ..
        } = &resolved
        {
            // frob:ticket 01M4YR00ZFABJQWKQJ3F3JFVY4
            let shown = path.to_string();
            let tail = shown.split_once('.').map_or("", |x| x.1);
            debug!(%path, %part, "N006: type read beside an instance");
            self.graph.diagnostics.push(DepDiagnostic {
                module: Some(ModuleKey::of_ctx(&ctx)),
                code: "N006",
                severity: Severity::Warning,
                message: format!(
                    "`{path}` reads the type, but this assembly has an instance; read `{part}.{tail}`"
                ),
                span: e.span,
                related: Vec::new(),
            });
        }
        let inst = &site.inst;
        let first = |p: &CanonPath| match p.members.first() {
            Some(MemberSeg::Name(n)) => Some(n.clone()),
            _ => None,
        };
        match resolved {
            Resolved::Member { path, .. } if path.parts.is_empty() => {
                let Some(m) = first(&path) else { return };
                if path.item_only() == site.comp_id {
                    out.insert(inst.key(&m));
                } else {
                    out.insert(node_key(&path.item_only(), &[], &m));
                }
            }
            Resolved::Member { path, .. } => {
                let Some(m) = first(&path) else { return };
                let mut parts = inst.parts.clone();
                parts.extend(path.parts.iter().map(|p| p.name.clone()));
                out.insert(node_key(&inst.root, &parts, &m));
                let slot = &path.parts[0].name;
                let array = comp.members.iter().any(|x| {
                    matches!(&x.kind, MemberKind::Part(p) if p.name.name == *slot && p.count.is_some())
                });
                if array {
                    out.insert(inst.key(slot));
                }
            }
            Resolved::Arg { name, .. } => {
                out.insert(inst.key(&name));
            }
            Resolved::Part { path } => {
                out.insert(inst.key(&path.parts[0].name));
            }
            Resolved::Local(_)
            | Resolved::Item { .. }
            | Resolved::Variant { .. }
            | Resolved::Prelude(_) => {}
        }
    }
}

impl DepGraph {
    fn insert(&mut self, key: CanonPath, span: Span, edges: BTreeSet<CanonPath>) {
        trace!(%key, edges = edges.len(), "node");
        self.nodes.insert(key, Node { span, edges });
    }
}

/// The member a call rolls up over parts: `sum(self, m)` or `m(self)`.
fn rollup_member(callee: &Expr, args: &[Arg]) -> Option<String> {
    let ExprKind::Path(cp) = &callee.kind else {
        return None;
    };
    let name = cp.segments.last()?.name.as_str();
    let is_self = |a: &Arg| matches!(&a.value.kind, ExprKind::Path(p) if p.text() == "self");
    match args {
        [a] if ROLLUP_NAMES.contains(&name) && is_self(a) => Some(name.to_owned()),
        [a, b] if ROLLUP_FNS.contains(&name) && is_self(a) => match &b.value.kind {
            ExprKind::Path(p) if p.segments.len() == 1 => Some(p.text()),
            _ => None,
        },
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolve::ModuleSrc;
    use cadetta_syntax::ast::File;
    use cadetta_syntax::parser::parse;

    fn file(src: &str) -> File {
        let p = parse(src);
        assert!(p.diags.is_empty(), "{:?}", p.diags);
        p.file
    }

    fn analyse(files: &[(&[&str], &File)]) -> DepGraph {
        let pkgs = vec![PackageSrc {
            name: "p".to_owned(),
            deps: Vec::new(),
            modules: files
                .iter()
                .map(|(path, f)| ModuleSrc {
                    path: path.iter().map(|s| (*s).to_owned()).collect(),
                    file: f,
                    enums: &[],
                })
                .collect(),
        }];
        let (resolver, errs) = Resolver::new(pkgs.clone());
        assert!(errs.is_empty());
        let renames = Renames::collect(&resolver, &pkgs);
        DepGraph::build(&resolver, &renames, &pkgs)
    }

    fn codes(g: &DepGraph) -> Vec<&'static str> {
        g.diagnostics().iter().map(|d| d.code).collect()
    }

    // frob:tests two_members_depending_on_each_other_are_d003
    #[test]
    fn two_members_depending_on_each_other_are_d003() {
        let f = file("model M {\n  a = b + 1\n  b = a + 1\n}\n");
        let g = analyse(&[(&[], &f)]);
        assert_eq!(codes(&g), vec!["D003"]);
        assert_eq!(g.diagnostics()[0].related.len(), 1);
        assert_eq!(g.diagnostics()[0].severity, Severity::Error);
        // Order of declaration is irrelevant.
        let f = file("model M {\n  b = a + 1\n  a = b + 1\n}\n");
        assert_eq!(codes(&analyse(&[(&[], &f)])), vec!["D003"]);
    }

    // frob:tests argument_through_joint_reading_same_model_is_no_cycle
    #[test]
    fn argument_through_joint_reading_same_model_is_no_cycle() {
        let f = file(
            "model Mount(hole: Length) {\n  thickness = 5mm\n  bore = hole\n}\n\
             model Screws(mount_t: Length) {\n  clearance_hole = mount_t + 1mm\n}\n\
             model Top {\n  part mount: Mount(hole = screws.clearance_hole)\n  \
             part screws: Screws(mount_t = mount.thickness)\n}\n",
        );
        let g = analyse(&[(&[], &f)]);
        assert!(codes(&g).is_empty(), "{:?}", g.diagnostics());
        // The same model with the member reading itself through the joint is a cycle.
        let f = file(
            "model Mount(hole: Length) {\n  thickness = hole + 1mm\n}\n\
             model Screws(mount_t: Length) {\n  clearance_hole = mount_t + 1mm\n}\n\
             model Top {\n  part mount: Mount(hole = screws.clearance_hole)\n  \
             part screws: Screws(mount_t = mount.thickness)\n}\n",
        );
        assert_eq!(codes(&analyse(&[(&[], &f)])), vec!["D003"]);
    }

    // frob:tests part_count_reading_member_depending_on_part_is_d003
    #[test]
    fn part_count_reading_member_depending_on_part_is_d003() {
        let f = file(
            "model Bolt { d = 4mm }\n\
             model Top {\n  n = count(bolts)\n  part bolts[n]: Bolt\n}\n",
        );
        assert_eq!(codes(&analyse(&[(&[], &f)])), vec!["D003"]);
        let f = file(
            "model Bolt { d = 4mm }\n\
             model Top {\n  n = 4\n  part bolts[n]: Bolt\n  total = sum(bolts.d)\n}\n",
        );
        assert!(codes(&analyse(&[(&[], &f)])).is_empty());
        let f = file(
            "model Bolt { d = 4mm }\n\
             model Top {\n  n = bolts.d\n  part bolts[n]: Bolt\n}\n",
        );
        assert_eq!(codes(&analyse(&[(&[], &f)])), vec!["D003"]);
    }

    // frob:tests rollup_over_parts_is_not_a_self_read
    #[test]
    fn rollup_over_parts_is_not_a_self_read() {
        let f = file(
            "model Bolt { mass = 4g }\n\
             model Top {\n  part bolts[2]: Bolt\n  mass = sum(self, mass)\n}\n",
        );
        assert!(codes(&analyse(&[(&[], &f)])).is_empty());
    }

    // frob:tests crates/cadetta-sema/src/deps.rs::DepGraph.build
    #[test]
    fn stale_item_name_in_was_is_n004() {
        let f = file(
            "model Part {\n  thickness = 3mm  was Old.thk\n}\nmodel Other {\n  t = Old.thk\n}\n",
        );
        assert_eq!(codes(&analyse(&[(&[], &f)])), vec!["N004"]);
        let f = file("model Part {\n  thickness = 3mm\n}\nmodel Other {\n  t = Part.thickness\n}\n");
        assert!(codes(&analyse(&[(&[], &f)])).is_empty());
    }

    // frob:tests crates/cadetta-sema/src/deps.rs::DepGraph.build
    #[test]
    fn type_read_beside_instance_is_n006() {
        let f = file(
            "model Plate {\n  thickness = 4mm\n}\n\
             assembly Rig {\n  part plate: Plate\n  total = Plate.thickness\n}\n",
        );
        let g = analyse(&[(&[], &f)]);
        assert_eq!(codes(&g), vec!["N006"]);
        assert!(g.diagnostics()[0].message.contains("`plate.thickness`"));
        let f = file(
            "model Plate {\n  thickness = 4mm\n}\n\
             assembly Rig {\n  part plate: Plate\n  total = plate.thickness\n}\n",
        );
        assert!(codes(&analyse(&[(&[], &f)])).is_empty());
    }

    // frob:tests crates/cadetta-sema/src/deps.rs::DepGraph.build
    #[test]
    fn lowercase_component_name_is_n001() {
        let f = file("model part_thing {\n  width = 3mm\n}\n");
        assert_eq!(codes(&analyse(&[(&[], &f)])), vec!["N001"]);
        let f = file("model PartThing {\n  width = 3mm\n}\n");
        assert!(codes(&analyse(&[(&[], &f)])).is_empty());
    }

    // frob:tests was_reference_resolves_with_n004
    #[test]
    fn was_reference_resolves_with_n004() {
        let f = file(
            "model Globals {\n  plate_thickness = 6mm was Globals.plate_thk\n}\n\
             model User {\n  t = Globals.plate_thk\n  u = Globals.plate_thickness\n}\n",
        );
        let g = analyse(&[(&[], &f)]);
        assert_eq!(codes(&g), vec!["N004"]);
        assert_eq!(g.diagnostics()[0].severity, Severity::Warning);
        let user = CanonPath::item("p", &[], "User");
        let t = node_key(&user, &[], "t");
        let new = node_key(
            &CanonPath::item("p", &[], "Globals"),
            &[],
            "plate_thickness",
        );
        assert!(g.deps_of(&t).any(|k| *k == new), "{:?}", g.nodes());

        // A moved module path is read textually.
        let moved = file("model Bracket {\n  t = 2mm was q::old::Bracket.thickness\n}\n");
        let user = file("model U {\n  v = q::old::Bracket.thickness\n}\n");
        let pkgs = vec![PackageSrc {
            name: "q".to_owned(),
            deps: Vec::new(),
            modules: vec![
                ModuleSrc {
                    path: vec!["frame".to_owned()],
                    file: &moved,
                    enums: &[],
                },
                ModuleSrc {
                    path: vec!["use_it".to_owned()],
                    file: &user,
                    enums: &[],
                },
            ],
        }];
        let (r, _) = Resolver::new(pkgs.clone());
        let g = DepGraph::build(&r, &Renames::collect(&r, &pkgs), &pkgs);
        assert_eq!(codes(&g), vec!["N004"]);
        let u = CanonPath::item("q", &["use_it".to_owned()], "U");
        let want = node_key(
            &CanonPath::item("q", &["frame".to_owned()], "Bracket"),
            &[],
            "t",
        );
        assert!(g.deps_of(&node_key(&u, &[], "v")).any(|k| *k == want));
    }

    // frob:tests graph_is_built_without_evaluating
    #[test]
    fn graph_is_built_without_evaluating() {
        // Division by zero and an undefined unit would fail evaluation; the graph does not care.
        let f = file("model M {\n  a = 1 / 0\n  b = a * 3furlong\n  c = b + a\n}\n");
        let g = analyse(&[(&[], &f)]);
        let m = CanonPath::item("p", &[], "M");
        let c = node_key(&m, &[], "c");
        let deps = g.deps_of(&c).cloned().collect::<Vec<_>>();
        assert_eq!(deps, vec![node_key(&m, &[], "a"), node_key(&m, &[], "b")]);
        assert!(codes(&g).is_empty());
    }
}
