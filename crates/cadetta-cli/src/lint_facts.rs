//! Facts the remaining lint families need and the IR does not carry (~A8J3XJW).
//!
//! Interfaces (I001), item visibility and use (U002, I004), `require` paths
//! (never suppressed) and `#[allow]` attributes (lints.md 2.2) are all read
//! from the parsed modules or the lowered IR here, so `pipeline` can hand them
//! to the `cadetta-lint` rule entry points.
//!
//! Specification: `docs/spec/lints.md` (I, U, 2.2, 2.3).

// frob:ticket 01M4TA3CABXHEZFA7BXA8J3XJW
// frob:ticket 01M4YEA3SADYRF5MJ4AG2DFAFQ

use std::collections::BTreeSet;

use cadetta_ir::canon::{CanonPath, MemberSeg};
use cadetta_ir::entities::{Interface, InterfaceBound, InterfaceMember};
use cadetta_ir::model::{Bound, BoundOp, Exact, Project, SpanRef};
use cadetta_ir::variant_name::ComponentPath;
use cadetta_lint::engine::Allow;
use cadetta_lint::rules::u002::{ItemFacts, ItemKind as FactKind, Visibility};
use cadetta_sema::resolve::{Ctx, Resolved, Resolver};
use cadetta_syntax::ast::{
    self, Arg, Attr, AttrArg, Block, BoundKind, Check, ComponentClass, Ctor, Expr, ExprKind, File,
    Fit, FitSpec, InterfaceMemberKind, Item, ItemKind, MemberKind, ParamDecl, ProcessMemberKind,
    Solver, SolverMemberKind, Span, Type, TypeKind, UnOp, Vis,
};
use cadetta_units::{Magnitude, Rational, UnitTable};
use tracing::{debug, trace};

/// One parsed module: its path inside the package, its AST and its source text.
pub struct ModuleView<'a> {
    /// Module segments below the package root.
    pub path: &'a [String],
    /// The parsed file.
    pub file: &'a File,
    /// The source text the file was parsed from.
    pub text: &'a str,
    /// The display name `SpanRef`s of this module carry.
    pub display: &'a str,
}

/// Lint visibility of an AST visibility modifier.
fn visibility(v: Vis) -> Visibility {
    match v {
        Vis::Private => Visibility::Private,
        Vis::PubPackage => Visibility::PubPackage,
        Vis::Pub => Visibility::Pub,
    }
}

/// Lower the `interface` items of `modules` (package `package`) to IR interfaces.
///
/// `extends` is flattened by last path segment within the same list, so an
/// implementer owes the members of every interface it inherits. Bounds with a
/// plain quantity limit are lowered to SI for C001/C002 (~J4802RH); a limit that
/// is not a literal is skipped with a debug log.
pub fn interfaces(package: &str, modules: &[ModuleView<'_>]) -> Vec<Interface> {
    let mut found: Vec<(Interface, Vec<String>)> = Vec::new();
    for m in modules {
        for item in m.file.items() {
            let ItemKind::Interface(i) = &item.kind else {
                continue;
            };
            let mut out = Interface {
                path: ComponentPath {
                    package: package.to_owned(),
                    modules: m.path.to_vec(),
                    item: i.name.name.clone(),
                },
                members: Vec::new(),
                bounds: Vec::new(),
                datums: Vec::new(),
            };
            for mem in &i.members {
                match &mem.kind {
                    InterfaceMemberKind::Field { name, optional, ty } => {
                        out.members.push(InterfaceMember {
                            name: name.name.clone(),
                            type_name: m
                                .text
                                .get(ty.span.start..ty.span.end)
                                .unwrap_or("")
                                .to_owned(),
                            optional: *optional,
                        });
                    }
                    InterfaceMemberKind::Datum(d) => out.datums.push(d.name.name.clone()),
                    InterfaceMemberKind::Bound(b) => out.bounds.extend(lower_bound(&b.kind)),
                    InterfaceMemberKind::Error => {}
                }
            }
            let parents = i
                .extends
                .iter()
                .filter_map(|p| p.segments.last().map(|s| s.name.clone()))
                .collect();
            tracing::debug!(interface = %i.name.name, members = out.members.len(), "lint_facts: interface lowered");
            found.push((out, parents));
        }
    }
    // Flatten inheritance; the passes bound the work on a cyclic `extends`.
    for _ in 0..found.len() {
        let snapshot = found.clone();
        for (iface, parents) in &mut found {
            for parent in parents.iter() {
                let Some((p, _)) = snapshot.iter().find(|(o, _)| &o.path.item == parent) else {
                    continue;
                };
                for mem in &p.members {
                    if !iface.members.iter().any(|x| x.name == mem.name) {
                        iface.members.push(mem.clone());
                    }
                }
                for d in &p.datums {
                    if !iface.datums.contains(d) {
                        iface.datums.push(d.clone());
                    }
                }
                for b in &p.bounds {
                    if !iface.bounds.contains(b) {
                        iface.bounds.push(b.clone());
                    }
                }
            }
        }
    }
    found.into_iter().map(|(i, _)| i).collect()
}

/// The exact SI magnitude of a single-segment quantity literal (`400N`, `-5mm`); `None` otherwise.
// frob:ticket 01M4V13T1DW08P87XP4J4802RH
fn literal_si(e: &Expr) -> Option<Exact> {
    match &e.kind {
        ExprKind::Paren(inner) => literal_si(inner),
        ExprKind::Unary(UnOp::Neg, inner) => {
            let m = literal_si(inner)?.0;
            Some(Exact(m.mul(&Magnitude::new(Rational::from_int(-1), 0))))
        }
        ExprKind::Quantity(segs) if segs.len() == 1 => {
            let n = Rational::parse_decimal(&segs[0].number).ok()?;
            let unit = UnitTable::builtin().parse(&segs[0].unit).ok()?;
            Some(Exact(Magnitude::new(n, 0).mul(&unit.factor)))
        }
        _ => {
            tracing::debug!("lint_facts: interface bound limit is not a plain quantity literal");
            None
        }
    }
}

/// Lower one interface bound to its IR form, when its limits are plain quantity literals.
// frob:ticket 01M4V13T1DW08P87XP4J4802RH
fn lower_bound(kind: &BoundKind) -> Option<InterfaceBound> {
    let (name, op, lo, hi) = match kind {
        BoundKind::Le { name, value } => (name, BoundOp::Le, None, Some(literal_si(value)?)),
        BoundKind::Ge { name, value } => (name, BoundOp::Ge, Some(literal_si(value)?), None),
        BoundKind::In { name, lo, hi } => (
            name,
            BoundOp::In,
            Some(literal_si(lo)?),
            Some(literal_si(hi)?),
        ),
        BoundKind::Eq { .. } => return None,
    };
    tracing::trace!(member = %name.name, ?op, "lint_facts: interface bound lowered");
    Some(InterfaceBound {
        name: name.name.clone(),
        bound: Bound { op, lo, hi },
    })
}

/// The declared name and fact kind of an item U002 tracks.
fn tracked(item: &Item) -> Option<(&str, FactKind)> {
    match &item.kind {
        ItemKind::Component(c) => Some((&c.name.name, FactKind::Component)),
        ItemKind::Fn(f) => Some((&f.name.name, FactKind::Function)),
        ItemKind::Interface(i) => Some((&i.name.name, FactKind::Interface)),
        ItemKind::Table(t) => Some((&t.name.name, FactKind::Table)),
        _ => None,
    }
}

/// What the checked package's own source refers to, found by name resolution (~G2DFAFQ).
#[derive(Default)]
struct Refs {
    /// Items named somewhere other than their own declaration.
    items: BTreeSet<CanonPath>,
    /// `(component, member)` pairs read somewhere.
    members: BTreeSet<(CanonPath, String)>,
    /// Components another component's `part` instantiates.
    instantiated: BTreeSet<CanonPath>,
}

/// Walks one item's syntax, resolving every reference through sema (locals shadow items).
struct Walker<'w, 'a> {
    resolver: &'w Resolver<'a>,
    refs: &'w mut Refs,
    ctx: Ctx,
    /// The item being walked; references to it from inside it are not "use".
    owner: CanonPath,
}

impl Walker<'_, '_> {
    /// Record what a resolved reference touches.
    fn record(&mut self, res: Option<Resolved>) {
        match res {
            Some(Resolved::Item { path, .. }) => self.mark_item(&path),
            Some(Resolved::Member { path, .. })
            | Some(Resolved::Arg { path, .. })
            | Some(Resolved::Part { path }) => self.mark_chain(&path),
            _ => {}
        }
    }

    /// Mark the item `p` names as used, unless it is the item being walked.
    fn mark_item(&mut self, p: &CanonPath) {
        let item = p.item_only();
        if item != self.owner {
            trace!(item = %item, from = %self.owner, "lint_facts: item referenced");
            self.refs.items.insert(item);
        }
    }

    /// Mark the item, every part component along the chain and the final member as used.
    fn mark_chain(&mut self, p: &CanonPath) {
        self.mark_item(p);
        let mut cur = p.item_only();
        for seg in &p.parts {
            let Some(comp) = self.resolver.component(&cur) else {
                return;
            };
            let slot = comp.members.iter().find_map(|m| match &m.kind {
                MemberKind::Part(pt) if pt.name.name == seg.name => Some(pt),
                _ => None,
            });
            let Some(slot) = slot else { return };
            trace!(owner = %cur, part = %seg.name, "lint_facts: part referenced");
            self.refs.members.insert((cur.clone(), seg.name.clone()));
            let Some((next, _)) = self
                .resolver
                .slot_component(&cur.package, &cur.modules, slot)
            else {
                return;
            };
            self.mark_item(&next);
            cur = next;
        }
        if let Some(MemberSeg::Name(n)) = p.members.first() {
            trace!(owner = %cur, member = %n, "lint_facts: member referenced");
            self.refs.members.insert((cur, n.clone()));
        }
    }

    /// Resolve a source path and record it.
    fn path(&mut self, p: &ast::Path) {
        match self.resolver.resolve_path(&self.ctx, p, None) {
            Ok(r) => self.record(Some(r)),
            Err(_) => self.own_datum(p),
        }
    }

    /// Record a read of the walked component's own datum, which sema does not resolve by bare name.
    // frob:ticket 01M4ZKRTB105N3QFA1RQ8C7WFF
    fn own_datum(&mut self, p: &ast::Path) {
        let (Some(first), Some(cname)) = (p.segments.first(), &self.ctx.component) else {
            return;
        };
        if self.ctx.locals.contains(&first.name) {
            return;
        }
        let cid = CanonPath::item(&self.ctx.package, &self.ctx.module, cname);
        let is_datum = self.resolver.component(&cid).is_some_and(|c| {
            c.members
                .iter()
                .any(|m| matches!(&m.kind, MemberKind::Datum(d) if d.name.name == first.name))
        });
        if is_datum {
            trace!(owner = %cid, datum = %first.name, "lint_facts: datum referenced");
            self.refs.members.insert((cid, first.name.clone()));
        }
    }

    /// Record the component a `part` or constructor instantiates.
    fn instantiate(&mut self, p: CanonPath) {
        if p != self.owner {
            trace!(component = %p, by = %self.owner, "lint_facts: component instantiated");
            self.refs.instantiated.insert(p);
        }
    }

    fn ty(&mut self, t: &Type) {
        match &t.kind {
            TypeKind::Path(p) => self.path(p),
            TypeKind::OnDatum { base, datum } => {
                self.path(base);
                self.path(datum);
            }
            TypeKind::Array { elem, len } => {
                self.ty(elem);
                self.expr(len);
            }
            TypeKind::Table(inner) => self.ty(inner),
            TypeKind::KindExpr(k) => k.terms.iter().for_each(|term| self.path(&term.name)),
        }
    }

    fn args(&mut self, args: &[Arg]) {
        for a in args {
            self.expr(&a.value);
        }
    }

    fn ctor(&mut self, c: &Ctor) {
        self.path(&c.path);
        if let Some((p, _)) = self
            .resolver
            .ctor_component(&self.ctx.package, &self.ctx.module, c)
        {
            self.instantiate(p);
        }
        if let Some(a) = &c.args {
            self.args(a);
        }
    }

    /// Walk the index expressions inside an already-resolved `a.b[i].c` chain.
    fn indices(&mut self, e: &Expr) {
        match &e.kind {
            ExprKind::Field(x, _) => self.indices(x),
            ExprKind::Index(x, i) => {
                self.indices(x);
                self.expr(i);
            }
            _ => {}
        }
    }

    fn expr(&mut self, e: &Expr) {
        match &e.kind {
            ExprKind::Number(_) | ExprKind::Quantity(_) | ExprKind::Str(_) | ExprKind::Bool(_) => {}
            ExprKind::Path(p) => self.path(p),
            ExprKind::Field(x, _) | ExprKind::Index(x, _) => {
                match self.resolver.resolve_ref(&self.ctx, e, None).ok() {
                    Some(r) => {
                        self.record(Some(r));
                        self.indices(e);
                    }
                    None => {
                        self.expr(x);
                        if let ExprKind::Index(_, i) = &e.kind {
                            self.expr(i);
                        }
                    }
                }
            }
            ExprKind::Paren(x) | ExprKind::Unary(_, x) | ExprKind::IsoClass(x, _) => self.expr(x),
            ExprKind::As(x, t) => {
                self.expr(x);
                self.ty(t);
            }
            ExprKind::Array(xs) => xs.iter().for_each(|x| self.expr(x)),
            ExprKind::Binary(_, a, b) | ExprKind::Satisfies(a, b) => {
                self.expr(a);
                self.expr(b);
            }
            ExprKind::Compare { first, rest } => {
                self.expr(first);
                rest.iter().for_each(|(_, x)| self.expr(x));
            }
            ExprKind::Call { callee, args, .. } => {
                match &callee.kind {
                    ExprKind::Path(p) => self.path(p),
                    _ => self.expr(callee),
                }
                self.args(args);
            }
            ExprKind::Tolerance(x, terms) => {
                self.expr(x);
                terms.iter().for_each(|t| self.expr(&t.value));
            }
            ExprKind::If {
                cond,
                then,
                otherwise,
            } => {
                self.expr(cond);
                self.block(then);
                if let Some(o) = otherwise {
                    self.expr(o);
                }
            }
            ExprKind::Match { scrutinee, arms } => {
                self.expr(scrutinee);
                arms.iter().for_each(|a| self.expr(&a.body));
            }
            ExprKind::Block(b) => self.block(b),
        }
    }

    /// Walk a block; its `let`s shadow items for the rest of the block.
    fn block(&mut self, b: &Block) {
        let mark = self.ctx.locals.len();
        for l in &b.lets {
            if let Some(t) = &l.ty {
                self.ty(t);
            }
            self.expr(&l.value);
            self.ctx.locals.push(l.name.name.clone());
        }
        self.expr(&b.tail);
        self.ctx.locals.truncate(mark);
    }

    fn param_decls(&mut self, params: &[ParamDecl]) {
        for p in params {
            self.ty(&p.ty);
            if let Some(d) = &p.default {
                self.expr(d);
            }
        }
    }

    fn bound(&mut self, b: &BoundKind) {
        match b {
            BoundKind::Le { value, .. } | BoundKind::Ge { value, .. } => self.expr(value),
            BoundKind::In { lo, hi, .. } => {
                self.expr(lo);
                self.expr(hi);
            }
            BoundKind::Eq { lhs, rhs } => {
                self.path(lhs);
                self.path(rhs);
            }
        }
    }

    fn check(&mut self, c: &Check) {
        let mark = self.ctx.locals.len();
        if let Some((var, over)) = &c.each {
            self.expr(over);
            self.ctx.locals.push(var.name.clone());
        }
        self.expr(&c.cond);
        self.ctx.locals.truncate(mark);
    }

    fn fit(&mut self, f: &Fit) {
        match &f.spec {
            FitSpec::Paths(a, b) => {
                self.path(a);
                self.path(b);
            }
            FitSpec::Class(_) | FitSpec::Pair(..) => {}
        }
        self.path(&f.on);
        if let Some((lo, hi)) = &f.range {
            self.expr(lo);
            self.expr(hi);
        }
    }

    fn solver(&mut self, s: &Solver) {
        for m in &s.members {
            match &m.kind {
                SolverMemberKind::In(binds) => binds.iter().for_each(|b| self.expr(&b.value)),
                SolverMemberKind::Out { ty, .. } => {
                    if let Some(t) = ty {
                        self.ty(t);
                    }
                }
                SolverMemberKind::Check(c) => self.check(c),
                SolverMemberKind::Deps(_) | SolverMemberKind::Error => {}
            }
        }
    }

    fn member(&mut self, kind: &MemberKind) {
        match kind {
            MemberKind::Param(p) => {
                if let Some(t) = &p.ty {
                    self.ty(t);
                }
                self.expr(&p.value);
            }
            MemberKind::Let(l) => {
                if let Some(t) = &l.ty {
                    self.ty(t);
                }
                self.expr(&l.value);
            }
            MemberKind::Bound(b) => self.bound(&b.kind),
            MemberKind::Part(p) => {
                if let Some(c) = &p.count {
                    self.expr(c);
                }
                self.ty(&p.ty);
                if let Some((cp, _)) =
                    self.resolver
                        .part_type(&self.ctx.package, &self.ctx.module, &p.ty)
                {
                    self.instantiate(cp);
                }
                if let Some(a) = &p.args {
                    self.args(a);
                }
                for c in p.primary.iter().chain(&p.alts) {
                    self.ctor(c);
                }
            }
            MemberKind::Fit(f) => self.fit(f),
            MemberKind::Check(c) => self.check(c),
            MemberKind::Require(r) => self.expr(&r.cond),
            MemberKind::Solver(s) => self.solver(s),
            MemberKind::Datum(d) => {
                if let Some(v) = &d.value {
                    self.expr(v);
                }
            }
            MemberKind::Place(p) => {
                self.expr(&p.target);
                self.expr(&p.at);
            }
            MemberKind::Error => {}
        }
    }

    fn item(&mut self, item: &Item) {
        match &item.kind {
            ItemKind::Component(c) => {
                c.implements.iter().for_each(|p| self.path(p));
                self.param_decls(&c.params);
                c.members.iter().for_each(|m| self.member(&m.kind));
            }
            ItemKind::Fn(f) => {
                self.param_decls(&f.params);
                self.ty(&f.ret);
                let mark = self.ctx.locals.len();
                self.ctx
                    .locals
                    .extend(f.params.iter().map(|p| p.name.name.clone()));
                self.block(&f.body);
                self.ctx.locals.truncate(mark);
            }
            ItemKind::Interface(i) => {
                i.extends.iter().for_each(|p| self.path(p));
                for m in &i.members {
                    match &m.kind {
                        InterfaceMemberKind::Field { ty, .. } => self.ty(ty),
                        InterfaceMemberKind::Bound(b) => self.bound(&b.kind),
                        InterfaceMemberKind::Datum(_) | InterfaceMemberKind::Error => {}
                    }
                }
            }
            ItemKind::Table(t) => {
                self.param_decls(t.key.as_slice());
                self.param_decls(&t.cols);
                for r in &t.rows {
                    self.expr(&r.key);
                    r.values.iter().flatten().for_each(|v| self.expr(v));
                }
            }
            ItemKind::Process(p) => {
                for m in &p.members {
                    match &m.kind {
                        ProcessMemberKind::Param(pm) => {
                            if let Some(t) = &pm.ty {
                                self.ty(t);
                            }
                            self.expr(&pm.value);
                        }
                        ProcessMemberKind::Rule(r) => {
                            self.param_decls(std::slice::from_ref(&r.param));
                            self.ty(&r.ret);
                            self.ctx.locals.push(r.param.name.name.clone());
                            self.expr(&r.body);
                            self.ctx.locals.pop();
                        }
                        ProcessMemberKind::Error => {}
                    }
                }
            }
            ItemKind::Solver(s) => self.solver(s),
            ItemKind::Fit(f) => self.fit(f),
            ItemKind::Require(r) => self.expr(&r.cond),
            ItemKind::Check(c) => self.check(c),
            ItemKind::Cite(_)
            | ItemKind::Enum(_)
            | ItemKind::Waive(_)
            | ItemKind::Unit(_)
            | ItemKind::Kind(_)
            | ItemKind::Error => {}
        }
    }
}

/// Resolve every reference in the checked package's modules (~G2DFAFQ).
fn collect_refs(package: &str, modules: &[ModuleView<'_>], resolver: &Resolver<'_>) -> Refs {
    let mut refs = Refs::default();
    for m in modules {
        for item in m.file.items() {
            let name = tracked(item).map_or("", |(n, _)| n);
            let mut w = Walker {
                resolver,
                refs: &mut refs,
                ctx: Ctx {
                    package: package.to_owned(),
                    module: m.path.to_vec(),
                    component: match &item.kind {
                        ItemKind::Component(c) => Some(c.name.name.clone()),
                        _ => None,
                    },
                    locals: Vec::new(),
                },
                owner: CanonPath::item(package, m.path, name),
            };
            w.item(item);
        }
    }
    debug!(
        package,
        items = refs.items.len(),
        members = refs.members.len(),
        instantiated = refs.instantiated.len(),
        "lint_facts: references resolved"
    );
    refs
}

/// U002 and I004 fact rows for the items and `priv` members of the checked package `package`.
///
/// `used` comes from name resolution (`collect_refs`), so a local or parameter that merely
/// shadows an item's name is not a use. `priv` parameter, `let`, `part` and `datum` members of a
/// component get `Member` rows. A component is `parameterized` when a model or assembly has a
/// parameter with no default, and a `is_root` when no other component's `part` instantiates
/// it. `instances` counts `part`s of the IR that instantiate a component.
// frob:ticket 01M4YEA3SADYRF5MJ4AG2DFAFQ
pub fn item_facts(
    package: &str,
    modules: &[ModuleView<'_>],
    ir: &Project,
    resolver: &Resolver<'_>,
) -> Vec<ItemFacts> {
    let refs = collect_refs(package, modules, resolver);
    let mut out = Vec::new();
    for m in modules {
        for item in m.file.items() {
            let Some((name, kind)) = tracked(item) else {
                continue;
            };
            let canon = CanonPath::item(package, m.path, name);
            let mut path = package.to_owned();
            for seg in m.path {
                path.push_str("::");
                path.push_str(seg);
            }
            path.push_str("::");
            path.push_str(name);
            let instances = ir
                .components
                .iter()
                .flat_map(|c| &c.parts)
                .filter(|p| p.primary.ends_with(&format!("::{name}")) || p.type_name == name)
                .count();
            let comp = match &item.kind {
                ItemKind::Component(c) => Some(c),
                _ => None,
            };
            let parameterized = comp.is_some_and(|c| {
                matches!(c.class, ComponentClass::Model | ComponentClass::Assembly)
                    && c.params.iter().any(|p| p.default.is_none() && !p.optional)
            });
            let is_root = comp.is_some() && !refs.instantiated.contains(&canon);
            let used = refs.items.contains(&canon);
            tracing::trace!(%path, used, parameterized, is_root, instances, "lint_facts: item fact");
            let span_of = |s: Span| SpanRef {
                file: m.display.to_owned(),
                start: s.start as u32,
                end: s.end as u32,
            };
            out.push(ItemFacts {
                span: Some(span_of(item.span)),
                path: path.clone(),
                kind,
                visibility: visibility(item.vis),
                in_checked_package: true,
                used,
                entry_point: matches!(comp, Some(c) if c.class == ComponentClass::Assembly),
                parameterized,
                is_root,
                instances,
            });
            for mem in comp.iter().flat_map(|c| &c.members).filter(|x| x.private) {
                let member = match &mem.kind {
                    MemberKind::Param(p) => &p.name.name,
                    MemberKind::Let(l) => &l.name.name,
                    MemberKind::Part(pt) => &pt.name.name,
                    MemberKind::Datum(d) => &d.name.name,
                    _ => continue,
                };
                let used = refs.members.contains(&(canon.clone(), member.clone()));
                tracing::trace!(%path, %member, used, "lint_facts: private member fact");
                out.push(ItemFacts {
                    path: format!("{path}.{member}"),
                    kind: FactKind::Member,
                    visibility: Visibility::Private,
                    in_checked_package: true,
                    used,
                    entry_point: false,
                    parameterized: false,
                    is_root: false,
                    instances: 0,
                    span: Some(span_of(mem.span)),
                });
            }
        }
    }
    out
}

/// `#[allow(CODE, "because")]` attributes of the lowered members (lints.md 2.2).
///
/// An allow without a reason is dropped here; the engine rejects it too.
pub fn allows(ir: &Project) -> Vec<Allow> {
    let mut out = Vec::new();
    for c in &ir.components {
        for m in &c.members {
            for (k, v) in &m.attrs {
                let Some(code) = k.strip_prefix("allow:") else {
                    continue;
                };
                let because = v.trim().trim_matches('"');
                match Allow::new(code, &m.path.to_string(), because) {
                    Ok(a) => out.push(a),
                    Err(error) => {
                        tracing::warn!(%code, path = %m.path, %error, "lint_facts: allow skipped");
                    }
                }
            }
        }
    }
    out
}

/// `#[allow(CODE, "because")]` on `check` and `require` members, read from the AST.
///
/// The lowered IR keeps no attributes for these statements, so `allows` cannot see them. A
/// `require` allow is collected too: the engine never suppresses it and W003 names it
/// (obligations.md 4; ~SN4G7QS).
// frob:ticket 01M4V1HYQACFV6PNTBESN4G7QS
pub fn statement_allows(package: &str, modules: &[ModuleView<'_>]) -> Vec<Allow> {
    let mut out = Vec::new();
    for m in modules {
        for item in m.file.items() {
            let ItemKind::Component(c) = &item.kind else {
                continue;
            };
            for member in &c.members {
                let name = match &member.kind {
                    MemberKind::Check(k) => &k.name.name,
                    MemberKind::Require(r) => &r.name.name,
                    _ => continue,
                };
                let mut path = package.to_owned();
                for seg in m.path {
                    path.push_str("::");
                    path.push_str(seg);
                }
                path.push_str(&format!("::{}.{name}", c.name.name));
                for a in member.attrs.iter().chain(&member.trailing) {
                    match statement_allow(m.text, a, &path) {
                        Some(Ok(allow)) => out.push(allow),
                        Some(Err(error)) => {
                            tracing::warn!(%path, %error, "lint_facts: statement allow skipped");
                        }
                        None => {}
                    }
                }
            }
        }
    }
    out
}

/// One `allow(CODE, "because")` attribute on the statement at `path`; `None` for other attributes.
fn statement_allow(
    text: &str,
    a: &Attr,
    path: &str,
) -> Option<Result<Allow, cadetta_lint::engine::EngineError>> {
    if a.name.name != "allow" {
        return None;
    }
    let arg_text = |g: &AttrArg| match g {
        AttrArg::Expr(e) => match &e.kind {
            ExprKind::Str(s) => s.clone(),
            _ => text.get(e.span.start..e.span.end).unwrap_or("").to_owned(),
        },
        AttrArg::Named { .. } => String::new(),
    };
    let code = a.args.first().map(arg_text)?;
    let because = a.args.get(1).map(arg_text).unwrap_or_default();
    tracing::trace!(%code, %path, "lint_facts: statement allow");
    Some(Allow::new(&code, path, &because))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lowered(src: &str) -> Vec<Interface> {
        let parsed = cadetta_syntax::parser::parse(src);
        let m = ModuleView {
            path: &[],
            file: &parsed.file,
            text: src,
            display: "t.cdt",
        };
        interfaces("fx", &[m])
    }

    // frob:tests crates/cadetta-cli/src/lint_facts.rs::interfaces
    #[test]
    fn interface_bounds_lower_to_si_and_inherit() {
        let ifs = lowered(
            "interface Base {\n  load >= 400N\n  dia in 2mm .. 5mm\n  gap == 3mm\n}\n\
             interface Child: Base {\n  mass <= 2kg\n}\n",
        );
        let base = ifs
            .iter()
            .find(|i| i.path.to_string().ends_with("Base"))
            .expect("base");
        assert_eq!(base.bounds.len(), 2, "Eq is skipped, Ge and In kept");
        assert_eq!(base.bounds[0].bound.op, BoundOp::Ge);
        assert!(base.bounds[0].bound.hi.is_none());
        assert_eq!(base.bounds[1].bound.op, BoundOp::In);
        let child = ifs
            .iter()
            .find(|i| i.path.to_string().ends_with("Child"))
            .expect("child");
        assert_eq!(
            child.bounds.len(),
            3,
            "own Le plus the two inherited bounds"
        );
    }
}
