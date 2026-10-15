//! Geometry in evaluation: datum, placement and re-expression values (geometry.md 2, 3, 4, 6).
//!
//! Sema owns no geometry math. This module adapts the syntax and the evaluator to
//! `cadetta-geometry`: the datum and placement members the geometry crate lowers are evaluated
//! like any parameter (memoised in the evaluator's table under their canonical paths), `p in D`
//! builds the geometry crate's frame table from those values and calls its `reexpress`, and
//! reads through part slots (`bracket.holes`) are resolved to the part type's own member.
//!
//! Decisions made here (written into geometry.md 3 and quantities.md 5.4): a re-expression runs
//! twice over the same frame table, once over the tolerance forms and once over the exact
//! nominals, so a rotation of a toleranced value keeps the nominal exact where the nominal is
//! exact and the form is an outer bound; the point form of a placement (`D.offset(p)`) reads its
//! x, y and z from the one point expression, selecting the instance by the member path of an
//! array placement; an indexed part instance (`leg[1].top in origin`) is not evaluated yet.

// frob:ticket 01M4VQXPDA4TEKE455FNYB5TXC

use std::collections::{BTreeMap, BTreeSet};

use cadetta_geometry::aligned::{AlignedError, AlignedReport, aligned};
use cadetta_geometry::axes;
use cadetta_geometry::datum::{
    ComponentId, DatumDef, GeometryError, Member as GeoMember, ORIGIN, lower_component_datums,
};
use cadetta_geometry::instances::{
    Operand, check_elementwise, check_unplaced, expand_instances, operands,
};
use cadetta_geometry::placement::{
    Assembly, Cover, Node, PlacementCode, PointInfo, build_assembly, check_assembly,
};
use cadetta_geometry::point::{DatumId, Point, Pos, point_type};
use cadetta_geometry::reexpress::{FrameTable, Located, ReexpressError, Reexpressed};
use cadetta_geometry::transform::Coord;
use cadetta_syntax::ast::{
    Arg, Component, ComponentClass, Expr, ExprKind, MemberKind, Part, Type, TypeKind,
};
use cadetta_syntax::span::Span;
use cadetta_units::compare::{Note, Verdict};
use cadetta_units::{Affine, Dim, RootOp};
use tracing::{debug, trace, warn};

use super::{
    EvalDiagnostic, Evaluated, Evaluator, Fail, Quant, R, Scope, Val, diag, func::Inex, owned,
    strip,
};
use crate::deps::Severity;
use crate::parts;
use crate::resolve::{CanonPath, Ctx, MemberSeg, ModuleKey, Resolver};

/// The datums and assembly tree of one component, as the geometry crate lowers them.
pub struct ComponentGeometry<'a> {
    /// Its datum declarations in order.
    pub datums: Vec<DatumDef>,
    /// Its parts and placements when it is an assembly.
    pub assembly: Option<Assembly>,
    /// Everything the geometry crate rejected while lowering (G-family emission is separate).
    pub errors: Vec<GeometryError>,
    /// The component itself.
    pub comp: &'a Component,
}

/// The id the geometry crate uses for a component path.
pub fn component_id(p: &CanonPath) -> ComponentId {
    ComponentId {
        package: p.package.clone(),
        modules: p.modules.clone(),
        item: p.item.clone(),
    }
}

fn item_path(id: &ComponentId) -> CanonPath {
    CanonPath::item(&id.package, &id.modules, &id.item)
}

/// The point info of a point-typed parameter or `let` written as `name`, `name[i]` or a chain
/// through part slots (`base.foot_sites`).
fn point_info(resolver: &Resolver<'_>, id: &ComponentId, e: &Expr) -> Option<PointInfo> {
    let chain = steps(e)?;
    let (last, slots) = chain.split_last()?;
    let mut comp = resolver.component(&item_path(id))?;
    let (mut package, mut modules) = (id.package.clone(), id.modules.clone());
    for (slot, _) in slots {
        let part = find_part(comp, slot)?;
        let (p, c) = resolver.part_type(&package, &modules, &part.ty)?;
        (package, modules, comp) = (p.package.clone(), p.modules.clone(), c);
    }
    let name = &last.0;
    let ty = comp.members.iter().find_map(|m| match &m.kind {
        MemberKind::Param(q) if q.name.name == *name => q.ty.as_ref(),
        MemberKind::Let(l) if l.name.name == *name => l.ty.as_ref(),
        _ => None,
    })?;
    let pt = point_type(unwrap_array(ty))?;
    Some(PointInfo {
        datum: pt.datum.0,
        arity: pt.kind.arity(),
    })
}

/// The element type of a (nested) array type; any other type is returned unchanged.
pub fn unwrap_array(ty: &Type) -> &Type {
    match &ty.kind {
        TypeKind::Array { elem, .. } => unwrap_array(elem),
        _ => ty,
    }
}

/// Lowers a component's datums, and its assembly tree when it has parts, through the geometry crate.
pub fn component_geometry<'a>(
    resolver: &Resolver<'a>,
    id: &ComponentId,
    comp: &'a Component,
) -> ComponentGeometry<'a> {
    let (datums, mut errors) = lower_component_datums(id, comp);
    let assembly = (comp.class == ComponentClass::Assembly).then(|| {
        let lookup = |from: &ComponentId, ty: &Type| {
            resolver
                .part_type(&from.package, &from.modules, ty)
                .map(|(p, c)| (component_id(&p), c))
        };
        let index = |e: &Expr| super::integer_literal(e).and_then(|n| u64::try_from(n).ok());
        let point_of = |from: &ComponentId, e: &Expr| point_info(resolver, from, e);
        let (a, es) = build_assembly(id, comp, &lookup, &index, &point_of);
        errors.extend(es);
        a
    });
    debug!(
        component = %id.item,
        datums = datums.len(),
        assembly = assembly.is_some(),
        errors = errors.len(),
        "component geometry lowered"
    );
    ComponentGeometry {
        datums,
        assembly,
        errors,
        comp,
    }
}

/// A reference written in an `in` expression, resolved syntactically through part slots.
enum Ref {
    /// A datum: the part slots crossed and the datum name.
    Datum { parts: Vec<String>, name: String },
    /// A declared point value: the part slots crossed and the datum its type is on.
    Value { parts: Vec<String>, tag: String },
}

/// Joins a part chain and a datum name into the frame table's id (`sub/bracket.base`).
fn datum_id(parts: &[String], name: &str) -> String {
    if parts.is_empty() {
        return name.to_owned();
    }
    let prefix = parts.join("/");
    match name.split_once('.') {
        Some(_) => format!("{prefix}/{name}"),
        None => format!("{prefix}.{name}"),
    }
}

/// Flattens `a.b[1].c` into steps; `None` for anything that is not a plain reference chain.
pub(super) fn steps(e: &Expr) -> Option<Vec<(String, Option<u64>)>> {
    match &strip(e).kind {
        ExprKind::Path(p) => Some(p.segments.iter().map(|s| (s.name.clone(), None)).collect()),
        ExprKind::Field(base, name) => {
            let mut out = steps(base)?;
            out.push((name.name.clone(), None));
            Some(out)
        }
        ExprKind::Index(base, idx) => {
            let mut out = steps(base)?;
            let n = super::integer_literal(idx).and_then(|n| u64::try_from(n).ok())?;
            let last = out.last_mut()?;
            if last.1.is_some() {
                return None;
            }
            last.1 = Some(n);
            Some(out)
        }
        _ => None,
    }
}

/// The `part` member named `name` of a component.
pub fn find_part<'c>(comp: &'c Component, name: &str) -> Option<&'c Part> {
    comp.members.iter().find_map(|m| match &m.kind {
        MemberKind::Part(p) if p.name.name == name => Some(p),
        _ => None,
    })
}

fn nominal_form(q: &Quant) -> Affine {
    Quant::constant(q.dim().clone(), q.nominal.clone()).form
}

/// Steps from component `cur` into the type of its part slot `name`; also whether it is counted.
fn step_part(
    resolver: &Resolver<'_>,
    cur: &CanonPath,
    name: &str,
) -> Result<(CanonPath, bool), String> {
    let comp = resolver
        .component(cur)
        .ok_or_else(|| format!("no component `{}`", cur.item))?;
    let part = find_part(comp, name).ok_or_else(|| format!("`{name}` is not a part"))?;
    let (next, _) = resolver
        .slot_component(&cur.package, &cur.modules, part)
        .ok_or_else(|| format!("the type of part `{name}` is unknown"))?;
    Ok((next, part.count.is_some()))
}

/// The definition-level path of a member read through part slots (indexes and counts ignored);
/// `None` when a slot does not resolve.
pub fn definition_path(resolver: &Resolver<'_>, path: &CanonPath) -> Option<CanonPath> {
    let mut cur = path.item_only();
    for seg in &path.parts {
        cur = step_part(resolver, &cur, &seg.name).ok()?.0;
    }
    cur.members = path.members.clone();
    Some(cur)
}

impl<'r, 'p> Evaluator<'r, 'p> {
    /// Resolves a read through part slots to the part type's own member (`bracket.holes`).
    pub(super) fn through_parts(&self, path: &CanonPath, span: Span) -> R<CanonPath> {
        if path.parts.iter().any(|s| s.index.is_some()) {
            return Err(diag(
                "EVAL",
                span,
                "reading through an indexed part instance is not evaluated yet",
            ));
        }
        let mut cur = path.item_only();
        for seg in &path.parts {
            let (next, counted) =
                step_part(self.resolver, &cur, &seg.name).map_err(|m| diag("EVAL", span, m))?;
            if counted {
                return Err(diag(
                    "EVAL",
                    span,
                    format!("`{}` is a part array; read one instance of it", seg.name),
                ));
            }
            cur = next;
        }
        cur.members = path.members.clone();
        trace!(from = %path, to = %cur, "read through parts resolved");
        Ok(cur)
    }

    /// `base[i]`: the i-th element of an evaluated array.
    pub(super) fn index(&mut self, sc: &mut Scope, base: &Expr, idx: &Expr, span: Span) -> R<Val> {
        let items = match self.eval(sc, base)? {
            Val::Array(items) => items,
            // frob:ticket 01M4TRBJ7D2JTGX6XX94BJ1ZGM
            Val::Table(table) => {
                let key = self.eval(sc, idx)?;
                return self.table_index(&table, key, span);
            }
            _ => {
                return Err(diag(
                    "EVAL",
                    span,
                    "only an array or a table can be indexed",
                ));
            }
        };
        let q = self.quant(sc, idx)?;
        let i = parts::array_count(&q)
            .map_err(|_| diag("EVAL", idx.span, "an index is a non-negative whole number"))?;
        items.get(i as usize).cloned().ok_or_else(|| {
            diag(
                "EVAL",
                span,
                format!("index {i} is out of range for an array of {}", items.len()),
            )
        })
    }

    /// Evaluates one datum or placement member, memoised under its canonical path.
    pub(super) fn geom_member(&mut self, m: &GeoMember) -> Option<Evaluated> {
        if let Some(v) = self.done.get(&m.path) {
            return v.clone();
        }
        if !self.in_progress.insert(m.path.clone()) {
            return None;
        }
        let mut sc = Scope {
            ctx: Ctx {
                package: m.path.package.clone(),
                module: m.path.modules.clone(),
                component: Some(m.path.item.clone()),
                locals: Vec::new(),
            },
            locals: Vec::new(),
        };
        let result = self
            .eval_param_expr(&mut sc, &m.value, m.value.span)
            .and_then(|ev| self.select_coordinate(m, ev));
        self.in_progress.remove(&m.path);
        let out = match result {
            Ok(v) => Some(v),
            Err(Fail::Silent) => None,
            Err(Fail::Diag(d)) => {
                self.diagnostics
                    .push(d.in_module(ModuleKey::of_path(&m.path)));
                None
            }
        };
        self.done.insert(m.path.clone(), out.clone());
        out
    }

    /// A point-form placement member holds the whole point; picks its instance and coordinate.
    fn select_coordinate(&self, m: &GeoMember, ev: Evaluated) -> R<Evaluated> {
        let span = m.value.span;
        let (mut value, mut picked) = (ev.value.clone(), false);
        if let Val::Array(items) = &value {
            let Some(i) = m.path.parts.iter().rev().find_map(|s| match s.index {
                Some(cadetta_ir::canon::Index::At(n)) => Some(n),
                _ => None,
            }) else {
                return Err(diag(
                    "EVAL",
                    span,
                    "an array of points needs an instance index",
                ));
            };
            value = items.get(i as usize).cloned().ok_or_else(|| {
                diag(
                    "EVAL",
                    span,
                    format!("instance {i} is out of range for the points"),
                )
            })?;
            picked = true;
        }
        if let Val::Point(cs) = &value {
            let axis = match m.path.members.last() {
                Some(MemberSeg::Name(n)) if n == "x" => 0,
                Some(MemberSeg::Name(n)) if n == "y" => 1,
                Some(MemberSeg::Name(n)) if n == "z" => 2,
                _ => return Err(diag("EVAL", span, "a point is read per axis")),
            };
            let c = cs
                .get(axis)
                .ok_or_else(|| diag("EVAL", span, "a Point2 has no z"))?;
            value = c.clone();
            picked = true;
        }
        if !picked {
            return Ok(ev);
        }
        Ok(owned(value, None, span))
    }

    /// Evaluates the datum and placement members of every component without parameters.
    pub(super) fn evaluate_geometry(&mut self) {
        let items = self.geom_components.clone();
        for item in items {
            let Some(comp) = self.resolver.component(&item) else {
                continue;
            };
            let id = component_id(&item);
            let start = self.diagnostics.len();
            let g = component_geometry(self.resolver, &id, comp);
            // frob:ticket 01M4V436F7JRA8P832RH0EKNH2
            self.report_geometry(&id, comp, &g);
            for d in &g.datums {
                for m in d.members() {
                    self.geom_member(m);
                }
            }
            if let Some(a) = &g.assembly {
                self.evaluate_placements(&id, comp, a);
            }
            self.stamp_since(start, &ModuleKey::of_path(&item));
        }
    }

    /// Emits G001 (lowering), G002, G003 and G006 (placement tree) and G007 (unplaced instances).
    // frob:ticket 01M4V436F7JRA8P832RH0EKNH2
    fn report_geometry(
        &mut self,
        id: &ComponentId,
        comp: &'p Component,
        g: &ComponentGeometry<'_>,
    ) {
        for e in &g.errors {
            if e.message.starts_with("G001") {
                warn!(error = %e, "geometry: G001");
                self.diagnostics.push(EvalDiagnostic {
                    module: None,
                    code: "G001",
                    severity: Severity::Error,
                    message: e.message.clone(),
                    span: e.span,
                });
            } else {
                debug!(error = %e, "geometry lowering reported");
            }
        }
        let mut shape = super::posshape::PosCheck::new(
            self.resolver,
            &id.package,
            &id.modules,
            comp,
            g.assembly.as_ref(),
        );
        shape.run();
        self.diagnostics.append(&mut shape.out);
        let Some(a) = &g.assembly else { return };
        for d in check_assembly(a) {
            let code = match d.code {
                PlacementCode::G002 => "G002",
                PlacementCode::G003 => "G003",
                PlacementCode::G006 => "G006",
                _ => {
                    debug!(diag = %d, "placement check: not a G-family finding here");
                    continue;
                }
            };
            self.diagnostics.push(EvalDiagnostic {
                module: None,
                code,
                severity: Severity::Error,
                message: d.to_string(),
                span: d.span,
            });
        }
        self.report_elementwise(id, comp, a);
        let mut counts: BTreeMap<String, u64> = BTreeMap::new();
        collect_counts(self, id, comp, a, &a.name, &mut counts);
        for d in check_unplaced(a, &|path: &str| counts.get(path).copied()) {
            self.diagnostics.push(EvalDiagnostic {
                module: None,
                code: "G007",
                severity: Severity::Warning,
                message: format!("G007: {}", d.message),
                span: d.span,
            });
        }
    }

    /// Emits G004 for array placements whose anchor operands do not match the part array.
    // frob:ticket 01M4VS6WG57MAD0T8GK956NGG9
    fn report_elementwise(&mut self, id: &ComponentId, comp: &'p Component, a: &Assembly) {
        let mut sc = Scope {
            ctx: Ctx {
                package: id.package.clone(),
                module: id.modules.clone(),
                component: Some(id.item.clone()),
                locals: Vec::new(),
            },
            locals: Vec::new(),
        };
        self.g004_nodes.clear();
        for p in &a.placements {
            let target = match (p.cover, find_part(comp, &p.node)) {
                (Cover::Whole, Some(part)) => self.part_count(id, part),
                _ => None,
            };
            let mut lens: Vec<(Operand, Option<u64>)> = Vec::new();
            for o in operands(p) {
                let n = match &o {
                    Operand::Point(e) => match self.eval(&mut sc, e) {
                        Ok(Val::Array(v)) => u64::try_from(v.len()).ok(),
                        _ => None,
                    },
                    Operand::Datum(node) => {
                        find_part(comp, node).and_then(|part| self.part_count(id, part))
                    }
                };
                lens.push((o, n));
            }
            let len = |o: &Operand| lens.iter().find(|(k, _)| k == o).and_then(|(_, n)| *n);
            for d in check_elementwise(p, target, &len) {
                warn!(node = %p.node, "geometry: G004 placement is not expanded");
                self.g004_nodes.insert(p.node.clone());
                self.diagnostics.push(EvalDiagnostic {
                    module: None,
                    code: "G004",
                    severity: Severity::Error,
                    message: d.to_string(),
                    span: d.span,
                });
            }
        }
    }

    fn evaluate_placements(&mut self, id: &ComponentId, comp: &'p Component, a: &Assembly) {
        for p in &a.placements {
            if self.g004_nodes.contains(&p.node) {
                debug!(node = %p.node, "geometry: skipping a placement that raised G004");
                continue;
            }
            let count = match (p.cover, find_part(comp, &p.node)) {
                (Cover::Whole, Some(part)) => self.part_count(id, part),
                _ => None,
            };
            if let Some(n) = count {
                let mut pp = item_path(id);
                pp.members.push(MemberSeg::Name(p.node.clone()));
                self.part_counts.insert(pp, n);
                for q in expand_instances(p, n) {
                    for m in q.members() {
                        self.geom_member(m);
                    }
                }
                if p.point.is_some() {
                    continue;
                }
            }
            for m in p.members() {
                self.geom_member(m);
            }
        }
    }

    /// The instance count of a `part name[N]` slot, when it has one and `N` evaluates.
    pub(super) fn part_count(&mut self, id: &ComponentId, part: &Part) -> Option<u64> {
        let e = part.count.as_ref()?;
        let mut sc = Scope {
            ctx: Ctx {
                package: id.package.clone(),
                module: id.modules.clone(),
                component: Some(id.item.clone()),
                locals: Vec::new(),
            },
            locals: Vec::new(),
        };
        let q = self.quant(&mut sc, e).ok()?;
        parts::array_count(&q).ok().map(u64::from)
    }

    /// Walks a reference chain from `comp`, crossing part slots; `Ok(None)` when it is not a
    /// declared datum or point (the caller reports).
    fn walk_ref(
        &self,
        comp: &'p Component,
        ctx: &Ctx,
        e: &Expr,
        span: Span,
    ) -> R<Option<(Ref, &'p Component)>> {
        let Some(chain) = steps(e) else {
            return Ok(None);
        };
        let mut cur = comp;
        let (mut module, mut package) = (ctx.module.clone(), ctx.package.clone());
        let mut crossed = Vec::new();
        for (k, (name, idx)) in chain.iter().enumerate() {
            if k + 1 < chain.len() {
                let Some(part) = find_part(cur, name) else {
                    return Ok(None);
                };
                if idx.is_some() {
                    return Err(diag(
                        "EVAL",
                        span,
                        "re-expression through an indexed part instance is not evaluated yet",
                    ));
                }
                let Some((p, c)) = self.resolver.part_type(&package, &module, &part.ty) else {
                    return Ok(None);
                };
                package = p.package;
                module = p.modules;
                crossed.push(name.clone());
                cur = c;
                continue;
            }
            if name == ORIGIN && idx.is_none() {
                return Ok(Some((
                    Ref::Datum {
                        parts: crossed,
                        name: name.clone(),
                    },
                    cur,
                )));
            }
            for m in &cur.members {
                match &m.kind {
                    MemberKind::Datum(d) if d.name.name == *name && idx.is_none() => {
                        return Ok(Some((
                            Ref::Datum {
                                parts: crossed,
                                name: name.clone(),
                            },
                            cur,
                        )));
                    }
                    MemberKind::Param(q) if q.name.name == *name => {
                        return self.value_ref(q.ty.as_ref(), crossed, cur, span);
                    }
                    MemberKind::Let(l) if l.name.name == *name => {
                        return self.value_ref(l.ty.as_ref(), crossed, cur, span);
                    }
                    _ => {}
                }
            }
            return Ok(None);
        }
        Ok(None)
    }

    fn value_ref(
        &self,
        ty: Option<&Type>,
        parts: Vec<String>,
        cur: &'p Component,
        span: Span,
    ) -> R<Option<(Ref, &'p Component)>> {
        let Some(pt) = ty.and_then(|t| point_type(unwrap_array(t))) else {
            return Err(diag(
                "EVAL",
                span,
                "`in` needs a point declared `Point2 on D` or `Point3 on D`",
            ));
        };
        Ok(Some((
            Ref::Value {
                parts,
                tag: pt.datum.0,
            },
            cur,
        )))
    }

    /// The datum name when `e` is a bare name that is a datum of one of `comp`'s part types.
    fn part_local_datum(&self, comp: &'p Component, ctx: &Ctx, e: &Expr) -> Option<String> {
        let chain = steps(e)?;
        let [(name, None)] = chain.as_slice() else {
            return None;
        };
        comp.members
            .iter()
            .filter_map(|m| match &m.kind {
                MemberKind::Part(p) => self.resolver.part_type(&ctx.package, &ctx.module, &p.ty),
                _ => None,
            })
            .any(|(_, c)| {
                c.members
                    .iter()
                    .any(|m| matches!(&m.kind, MemberKind::Datum(d) if d.name.name == *name))
            })
            .then(|| name.clone())
    }

    /// `aligned(a, b, allowance)` as a check condition (geometry.md 5): the verdict and notes.
    ///
    /// The pairing, the datum conversion and the tolerance bound are `cadetta_geometry::aligned`;
    /// this adapts the operands (declared point arrays) and maps its errors to G001, G003, G004.
    // frob:ticket 01M4VS6WG57MAD0T8GK956NGG9
    pub(super) fn aligned_condition(
        &mut self,
        sc: &mut Scope,
        args: &[Arg],
        span: Span,
    ) -> R<(Verdict, Vec<Note>)> {
        let xs = super::func::arity("aligned", args, 3, span)?;
        let allowance = self.quant(sc, xs[2])?;
        // Inside an instance, `at` and `onto` are arguments: read them where the caller wrote them.
        let (a, b, here, comp) = match (self.caller_operand(xs[0]), self.caller_operand(xs[1])) {
            (Some((ea, ctx)), Some((eb, _))) => {
                trace!("geometry: aligned operands read from the instance's caller");
                let mut caller = Scope {
                    ctx,
                    locals: Vec::new(),
                };
                let (here, comp) = self.enclosing("aligned", &caller, span)?;
                let a = self.point_operand(&mut caller, comp, ea)?;
                let b = self.point_operand(&mut caller, comp, eb)?;
                (a, b, here, comp)
            }
            _ => {
                let (here, comp) = self.enclosing("aligned", sc, span)?;
                let a = self.point_operand(sc, comp, xs[0])?;
                let b = self.point_operand(sc, comp, xs[1])?;
                (a, b, here, comp)
            }
        };
        let (table, _) = self.frame_tables(&here, comp, span)?;
        let report =
            aligned(&table, &a, &b, &allowance.form).map_err(|e| aligned_diag(&e, span))?;
        debug!(verdict = ?report.verdict, pairs = report.pairs.len(), "geometry: aligned condition");
        Ok((report.verdict, report_notes(&report)))
    }

    /// A bare callee argument name as the caller's expression and context (inside an instance only).
    // frob:ticket 01M4SWG6XQ2M0GR620Q6MJE3XF
    fn caller_operand(&self, e: &Expr) -> Option<(&'p Expr, Ctx)> {
        let inst = self.cur_inst.as_ref()?;
        let chain = steps(e)?;
        let [(name, None)] = chain.as_slice() else {
            return None;
        };
        let mut key = inst.prefix.clone();
        key.members = vec![MemberSeg::Name(name.clone())];
        let def = self.defs.get(&key)?;
        if def.inst.is_some() {
            return None;
        }
        Some((def.expr, def.ctx.clone()))
    }

    /// The enclosing component of a geometry builtin, as its path and syntax.
    fn enclosing(&mut self, what: &str, sc: &Scope, span: Span) -> R<(CanonPath, &'p Component)> {
        let Some(comp_name) = sc.ctx.component.clone() else {
            return Err(diag(
                "EVAL",
                span,
                format!("`{what}` is evaluated inside a component"),
            ));
        };
        let here = CanonPath::item(&sc.ctx.package, &sc.ctx.module, &comp_name);
        let comp = self
            .resolver
            .component(&here)
            .ok_or_else(|| diag("EVAL", span, "the enclosing component is unknown"))?;
        Ok((here, comp))
    }

    /// A datum operand of an axis builtin (`a.ax`): its datum id, G003 for a part-local name.
    fn axis_operand(&mut self, sc: &Scope, comp: &'p Component, e: &Expr) -> R<DatumId> {
        match self.walk_ref(comp, &sc.ctx, e, e.span)? {
            Some((Ref::Datum { parts, name }, _)) => Ok(DatumId::new(datum_id(&parts, &name))),
            Some(_) => Err(diag("EVAL", e.span, "an axis operand names a datum")),
            None => {
                if let Some(name) = self.part_local_datum(comp, &sc.ctx, e) {
                    return Err(diag(
                        "G003",
                        e.span,
                        format!(
                            "G003: `{name}` is a part-local datum met in an assembly; name it as `part.{name}` (geometry.md 3)"
                        ),
                    ));
                }
                Err(diag("EVAL", e.span, "an axis operand names a datum"))
            }
        }
    }

    /// `coaxial(a, b, allowance, over)` or `parallel(..)` as a check condition (geometry.md 5.1).
    ///
    /// The axis sampling, the carry between datums and the tolerance bound are
    /// `cadetta_geometry::axes`; this adapts the operands and maps its errors to G001, G003.
    // frob:ticket 01M4V43BJ1Z4EZTYJXMMCJWA2D
    pub(super) fn axis_condition(
        &mut self,
        sc: &mut Scope,
        name: &str,
        args: &[Arg],
        span: Span,
    ) -> R<(Verdict, Vec<Note>)> {
        let xs = super::func::arity(name, args, 4, span)?;
        let (here, comp) = self.enclosing(name, sc, span)?;
        let a = self.axis_operand(sc, comp, xs[0])?;
        let b = self.axis_operand(sc, comp, xs[1])?;
        let allowance = self.quant(sc, xs[2])?;
        let over = self.quant(sc, xs[3])?;
        let (table, _) = self.frame_tables(&here, comp, span)?;
        let report = if name == "coaxial" {
            axes::coaxial(&table, &a, &b, &allowance.form, &over.form)
        } else {
            axes::parallel(&table, &a, &b, &allowance.form, &over.form)
        }
        .map_err(|e| aligned_diag(&e, span))?;
        debug!(name, verdict = ?report.verdict, "geometry: axis condition");
        Ok((report.verdict, report_notes(&report)))
    }

    /// `angle_between(a, b)`: the angle between two datum z axes, exact or marked inexact (Q006).
    // frob:ticket 01M4V43BJ1Z4EZTYJXMMCJWA2D
    pub(super) fn angle_between(&mut self, sc: &mut Scope, args: &[Arg], span: Span) -> R<Val> {
        let xs = super::func::arity("angle_between", args, 2, span)?;
        let (here, comp) = self.enclosing("angle_between", sc, span)?;
        let a = self.axis_operand(sc, comp, xs[0])?;
        let b = self.axis_operand(sc, comp, xs[1])?;
        let (form, nominal) = self.frame_tables(&here, comp, span)?;
        let r = axes::angle_between(&form, &nominal, &a, &b).map_err(|e| aligned_diag(&e, span))?;
        let dim = Dim::new(cadetta_units::Dimension::dimensionless(), 1);
        if let Some(m) = r.exact {
            debug!("geometry: angle_between is exact");
            return Ok(Val::Quantity(Quant::constant(dim, m)));
        }
        debug!(
            lo = r.lo,
            hi = r.hi,
            "geometry: angle_between is inexact (Q006)"
        );
        Ok(Val::Inexact(Inex {
            dim,
            nominal: r.nominal,
            lo: r.lo,
            hi: r.hi,
            root: RootOp::Atan2,
            exact_nominal: None,
        }))
    }

    /// A declared point or point array (`lid.holes`) as a position in its own datum.
    fn point_operand(&mut self, sc: &mut Scope, comp: &'p Component, e: &Expr) -> R<Pos> {
        let Some((Ref::Value { parts, tag }, _)) = self.walk_ref(comp, &sc.ctx, e, e.span)? else {
            return Err(diag(
                "EVAL",
                e.span,
                "`aligned` takes declared points or point arrays",
            ));
        };
        let val = self.eval(sc, e)?;
        to_pos(&val, &DatumId::new(datum_id(&parts, &tag)), false, e.span)
    }

    /// The frame tables of `comp` over the tolerance forms and over the exact nominals.
    fn frame_tables(
        &mut self,
        here: &CanonPath,
        comp: &'p Component,
        span: Span,
    ) -> R<(FrameTable, FrameTable)> {
        let id = component_id(here);
        let g = component_geometry(self.resolver, &id, comp);
        let scope_assembly = comp.class == ComponentClass::Assembly
            || comp
                .members
                .iter()
                .any(|m| matches!(m.kind, MemberKind::Part(_)));
        let mut members: Vec<GeoMember> = g
            .datums
            .iter()
            .flat_map(|d| d.members().into_iter().cloned())
            .collect();
        let mut asm = g.assembly.clone();
        if let Some(a) = asm.as_mut() {
            self.prune_assembly(&id, comp, a);
            collect_members(a, &mut members);
        }
        let mut forms: BTreeMap<CanonPath, Affine> = BTreeMap::new();
        let mut nominals: BTreeMap<CanonPath, Affine> = BTreeMap::new();
        for m in &members {
            if let Some(Evaluated {
                value: Val::Quantity(q),
                ..
            }) = self.geom_member(m)
            {
                nominals.insert(m.path.clone(), nominal_form(&q));
                forms.insert(m.path.clone(), q.form);
            }
        }
        let lookup = |table: &BTreeMap<CanonPath, Affine>| {
            let table = table.clone();
            move |m: &GeoMember| -> Result<Affine, GeometryError> {
                table.get(&m.path).cloned().ok_or(GeometryError {
                    span: m.value.span,
                    message: format!("`{}` failed to evaluate", m.path),
                })
            }
        };
        let (form_eval, nominal_eval) = (lookup(&forms), lookup(&nominals));
        let build = |eval: &dyn Fn(&GeoMember) -> Result<Affine, GeometryError>| match (
            &asm,
            scope_assembly,
        ) {
            (Some(a), true) => FrameTable::for_assembly(a, eval),
            _ => FrameTable::for_part(&g.datums, eval),
        };
        let (ft_form, ft_nominal) = match (build(&form_eval), build(&nominal_eval)) {
            (Ok(f), Ok(n)) => (f, n),
            (Err(e), _) | (_, Err(e)) => return Err(reexpress_diag(&e, span)),
        };
        Ok((ft_form, ft_nominal))
    }

    /// `p in D` (geometry.md 3): re-expresses a point, or an array of points, in datum `D`.
    pub(super) fn reexpress(&mut self, sc: &mut Scope, l: &Expr, r: &Expr, span: Span) -> R<Val> {
        let Some(comp_name) = sc.ctx.component.clone() else {
            return Err(diag("EVAL", span, "`in` is evaluated inside a component"));
        };
        let here = CanonPath::item(&sc.ctx.package, &sc.ctx.module, &comp_name);
        let comp = self
            .resolver
            .component(&here)
            .ok_or_else(|| diag("EVAL", span, "the enclosing component is unknown"))?;
        let Some((right, _)) = self.walk_ref(comp, &sc.ctx, r, r.span)? else {
            // frob:ticket 01M4VQXPDA4TEKE455FNYB5TXC
            if let Some(name) = self.part_local_datum(comp, &sc.ctx, r) {
                return Err(diag(
                    "G003",
                    r.span,
                    format!(
                        "G003: `{name}` is a part-local datum met in an assembly; name it as `part.{name}` (geometry.md 3)"
                    ),
                ));
            }
            return Err(diag("EVAL", r.span, "the right side of `in` names a datum"));
        };
        let Ref::Datum {
            parts: rparts,
            name: rname,
        } = right
        else {
            return Err(diag("EVAL", r.span, "the right side of `in` names a datum"));
        };
        let to = DatumId::new(datum_id(&rparts, &rname));
        let (left_val, from_tag) = match self.walk_ref(comp, &sc.ctx, l, l.span)? {
            Some((Ref::Datum { parts, name }, _)) => {
                let id = datum_id(&parts, &name);
                let zero = Val::Quantity(Quant::constant(
                    Dim::new(cadetta_units::Dimension::from_si([1, 0, 0, 0, 0, 0, 0]), 0),
                    cadetta_units::Magnitude::new(cadetta_units::Rational::from_int(0), 0),
                ));
                (Val::Point(vec![zero.clone(), zero.clone(), zero]), id)
            }
            Some((Ref::Value { parts, tag }, _)) => (self.eval(sc, l)?, datum_id(&parts, &tag)),
            None => {
                return Err(diag(
                    "EVAL",
                    l.span,
                    "the left side of `in` is a declared point or a datum",
                ));
            }
        };
        let (ft_form, ft_nominal) = self.frame_tables(&here, comp, span)?;
        let from = DatumId::new(from_tag);
        let run = |ft: &FrameTable, nominal: bool| -> R<Reexpressed> {
            let pos = to_pos(&left_val, &from, nominal, l.span)?;
            ft.reexpress(&pos, &to)
                .map_err(|e| reexpress_diag(&e, span))
        };
        let (rf, rn) = (run(&ft_form, false)?, run(&ft_nominal, true)?);
        merge(&rf, &rn, span)
    }

    /// Drops the placements the table cannot evaluate: a point-form placement of a part array.
    fn prune_assembly(&mut self, id: &ComponentId, comp: &Component, a: &mut Assembly) {
        let counted: BTreeSet<String> = comp
            .members
            .iter()
            .filter_map(|m| match &m.kind {
                MemberKind::Part(p) if p.count.is_some() => Some(p.name.name.clone()),
                _ => None,
            })
            .collect();
        a.placements.retain(|p| {
            !(p.point.is_some() && counted.contains(&p.node) && p.cover == Cover::Whole)
        });
        for n in a.nodes.iter_mut() {
            let Some(sub) = n.sub.as_mut() else { continue };
            let Some(part) = find_part(comp, &n.name) else {
                continue;
            };
            if let Some((p, c)) = self.resolver.part_type(&id.package, &id.modules, &part.ty) {
                self.prune_assembly(&component_id(&p), c, sub);
            }
        }
    }
}

/// Every datum member of every node and every placement member of `a`, recursively.
fn collect_members(a: &Assembly, out: &mut Vec<GeoMember>) {
    for n in &a.nodes {
        collect_node(n, out);
    }
    for p in &a.placements {
        out.extend(p.members().into_iter().cloned());
    }
}

fn collect_node(n: &Node, out: &mut Vec<GeoMember>) {
    for d in &n.datums {
        out.extend(d.members().into_iter().cloned());
    }
    if let Some(sub) = &n.sub {
        collect_members(sub, out);
    }
}

/// The distinct comparison notes across every measured pair of a report.
fn report_notes(report: &AlignedReport) -> Vec<Note> {
    let mut notes: Vec<Note> = Vec::new();
    for m in &report.pairs {
        for n in &m.comparison.notes {
            if !notes.contains(n) {
                notes.push(*n);
            }
        }
    }
    notes
}

fn aligned_diag(e: &AlignedError, span: Span) -> Fail {
    let code = match e.code() {
        Some("G001") => "G001",
        Some("G003") => "G003",
        Some("G004") => "G004",
        _ => "EVAL",
    };
    diag(code, span, e.to_string())
}

fn reexpress_diag(e: &ReexpressError, span: Span) -> Fail {
    let code = match e.code() {
        Some("G003") => "G003",
        _ => "EVAL",
    };
    diag(code, span, e.to_string())
}

fn to_pos(v: &Val, datum: &DatumId, nominal: bool, span: Span) -> R<Pos> {
    match v {
        Val::Point(cs) => {
            let mut coords = Vec::with_capacity(cs.len());
            for c in cs {
                match c {
                    Val::Quantity(q) => coords.push(if nominal {
                        nominal_form(q)
                    } else {
                        q.form.clone()
                    }),
                    _ => {
                        return Err(diag(
                            "EVAL",
                            span,
                            "an inexact coordinate cannot be re-expressed",
                        ));
                    }
                }
            }
            Ok(Pos::Point(Point {
                datum: datum.clone(),
                coords,
            }))
        }
        Val::Array(xs) => xs
            .iter()
            .map(|x| to_pos(x, datum, nominal, span))
            .collect::<R<Vec<_>>>()
            .map(Pos::Array),
        _ => Err(diag("EVAL", span, "`in` re-expresses points")),
    }
}

fn merge(form: &Reexpressed, nominal: &Reexpressed, span: Span) -> R<Val> {
    match (form, nominal) {
        (Reexpressed::Point(f), Reexpressed::Point(n)) => merge_point(f, n),
        (Reexpressed::Array(fs), Reexpressed::Array(ns)) => fs
            .iter()
            .zip(ns)
            .map(|(f, n)| merge(f, n, span))
            .collect::<R<Vec<_>>>()
            .map(Val::Array),
        _ => Err(diag("EVAL", span, "re-expression shapes disagree")),
    }
}

fn merge_point(f: &Located, n: &Located) -> R<Val> {
    let mut out = Vec::with_capacity(f.coords.len());
    for (fc, nc) in f.coords.iter().zip(&n.coords) {
        out.push(match (fc, nc) {
            (Coord::Exact(form), Coord::Exact(nom)) => Val::Quantity(Quant {
                form: form.clone(),
                nominal: nom.center().clone(),
            }),
            (Coord::Approx(a), nc) => {
                let exact_nominal = match nc {
                    Coord::Exact(nom) => Some(nom.center().clone()),
                    Coord::Approx(_) => None,
                };
                Val::Inexact(Inex {
                    dim: a.dim.clone(),
                    nominal: (a.lo + a.hi) / 2.0,
                    lo: a.lo,
                    hi: a.hi,
                    root: a.root.unwrap_or(RootOp::MixedPiSum),
                    exact_nominal,
                })
            }
            (Coord::Exact(_), Coord::Approx(_)) => {
                warn!("nominal run inexact while the form run is exact");
                return Err(diag(
                    "EVAL",
                    Span::default(),
                    "a re-expressed coordinate is exact in its form but not its nominal",
                ));
            }
        });
    }
    Ok(Val::Point(out))
}

/// Fills `out` with the instance count of every counted part slot of `a`, by instance path.
fn collect_counts<'p>(
    ev: &mut Evaluator<'_, 'p>,
    id: &ComponentId,
    comp: &'p Component,
    a: &Assembly,
    prefix: &str,
    out: &mut BTreeMap<String, u64>,
) {
    for n in &a.nodes {
        let path = format!("{prefix}/{}", n.name);
        let Some(part) = find_part(comp, &n.name) else {
            continue;
        };
        if let Some(c) = ev.part_count(id, part) {
            out.insert(path.clone(), c);
        }
        if let Some(sub) = &n.sub
            && let Some((p, c)) = ev.resolver.part_type(&id.package, &id.modules, &part.ty)
        {
            collect_counts(ev, &component_id(&p), c, sub, &path, out);
        }
    }
}
