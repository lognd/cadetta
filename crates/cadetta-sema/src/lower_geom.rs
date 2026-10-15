// frob:ticket 01M4VQXPDA4TEKE455FNYB5TXC
//! Lowering of datums, points and placements into the IR (ir.md 3; geometry.md 2, 4, 6).
//!
//! The structure comes from `cadetta-geometry` (`component_geometry`, `datum_members`,
//! `op_members`, `expand_instances`); the values come from the evaluation the evaluator already
//! made of those members. Decisions made here (written into ir.md 3): a placement's `datum` and
//! `at` name the definition-level datum of the part type (`Type.datum`), so the digest graph has
//! one node per datum however many slots share a type; an array placement lowers to one
//! Placement per instance with integer indexes and an unknown count skips it; a point's
//! coordinates are Members `Type.p.x` (or `Type.holes[i].x`) of kind `let`, derived from the
//! member that declares the point, and the Point's `position` is left absent.

use cadetta_geometry::datum::{Angle, DatumDef, DatumOp, ORIGIN};
use cadetta_geometry::instances::expand_instances;
use cadetta_geometry::op_members::{OpSource, datum_members, ir_path as geo_path, op_members};
use cadetta_geometry::placement::{Anchor, Cover, Placement as GeoPlacement};
use cadetta_geometry::point::point_type;
use cadetta_ir::canon::{Index, PartSeg};
use cadetta_ir::geometry::{Axis as IrAxis, Datum, OffsetArgs, Op, Placement, Point};
use cadetta_ir::model::{Component, Member, MemberKind, Provenance, Value};
use cadetta_ir::path::Path;
use cadetta_syntax::ast;
use tracing::{debug, trace, warn};

use super::{Cx, SourceText, evaluated_value, text_of};
use crate::eval::geom::{component_geometry, component_id, find_part, unwrap_array};
use crate::eval::{GeometryKind, Val};
use crate::resolve::{CanonPath, MemberSeg};

fn axis(a: cadetta_geometry::datum::Axis) -> IrAxis {
    match a {
        cadetta_geometry::datum::Axis::X => IrAxis::X,
        cadetta_geometry::datum::Axis::Y => IrAxis::Y,
        cadetta_geometry::datum::Axis::Z => IrAxis::Z,
    }
}

/// The IR path of a canonical path, logging and dropping one that is not well formed.
fn path_of(p: &CanonPath) -> Option<Path> {
    match geo_path(p) {
        Ok(p) => Some(p),
        Err(e) => {
            warn!(path = %p, error = %e, "lower: geometry path is not a valid IR path");
            None
        }
    }
}

/// The IR operations of a datum or placement chain.
fn ir_ops(ops: &[DatumOp]) -> Option<Vec<Op>> {
    let mut out = Vec::new();
    for op in ops {
        out.push(match op {
            DatumOp::Offset { args } => {
                let mut o = OffsetArgs::default();
                for (a, m) in args {
                    let slot = match axis(*a) {
                        IrAxis::X => &mut o.x,
                        IrAxis::Y => &mut o.y,
                        IrAxis::Z => &mut o.z,
                    };
                    *slot = Some(path_of(&m.path)?);
                }
                Op::Offset(Box::new(o))
            }
            DatumOp::Rotate {
                axis: a,
                angle: Angle::Given(m),
            } => Op::Rotate {
                axis: axis(*a),
                angle: path_of(&m.path)?,
            },
            DatumOp::Rotate {
                axis: a,
                angle: Angle::HalfTurn,
            } => Op::Flip { axis: axis(*a) },
        });
    }
    Some(out)
}

/// `Type.name` for a datum of a component type.
fn datum_path(ty: &CanonPath, name: &str) -> CanonPath {
    let mut p = ty.item_only();
    p.members.push(MemberSeg::Name(name.to_owned()));
    p
}

/// `Type/a/b.d` for a possibly dotted datum text (`a.d` is datum `d` of the sub-part `a`).
fn dotted_datum_path(ty: &CanonPath, text: &str) -> CanonPath {
    let mut names: Vec<&str> = text.split('.').collect();
    let last = names.pop().unwrap_or(ORIGIN);
    let mut p = ty.item_only();
    p.parts = names
        .into_iter()
        .map(|n| PartSeg {
            name: n.to_owned(),
            index: None,
        })
        .collect();
    p.members.push(MemberSeg::Name(last.to_owned()));
    p
}

impl Cx<'_> {
    /// The datum and placement argument members of the parameterized instances declared in
    /// `comp` (`Box/j.mount.offset.x`), valued from the per-instance evaluation.
    // frob:ticket 01M4VYZS8VGT1RASS407Y8K809
    pub(super) fn lower_instance_geometry(&self, comp: &CanonPath) -> Vec<Member> {
        let ev = self.input.evaluation;
        let mut out = Vec::new();
        for (k, kind) in &ev.instance_geometry {
            if k.item_only() != *comp {
                continue;
            }
            let (Some(e), Some(path)) = (ev.get(k), path_of(k)) else {
                continue;
            };
            let kind = match kind {
                GeometryKind::Datum => MemberKind::Datum,
                GeometryKind::Place => MemberKind::Place,
            };
            let mut m = Member::new(path, kind);
            m.provenance = Provenance::Derived;
            self.fill_geometry_value(&mut m, k);
            m.analysis = m.analysis || e.analysis;
            out.push(m);
        }
        trace!(component = %comp, members = out.len(), "lower: instance geometry members");
        out
    }

    /// Fills a geometry member's value and variants from the evaluation of `canon`.
    fn fill_geometry_value(&self, out: &mut Member, canon: &CanonPath) {
        let ev = self.input.evaluation;
        let text = out.expr.clone().unwrap_or_default();
        match ev.get(canon) {
            Some(e) => {
                out.value = evaluated_value(ev, &self.owners, canon, e, &text);
                out.analysis = out.analysis || e.analysis;
            }
            None => trace!(path = %canon, "lower: geometry member has no evaluated value"),
        }
        out.variants = self.differing_variants(canon, &text, out.value.as_ref());
    }

    /// Lowers the datums, placements and points of component `c` into `comp`.
    pub(super) fn lower_geometry(
        &self,
        canon: &CanonPath,
        src: Option<&SourceText<'_>>,
        c: &ast::Component,
        comp: &mut Component,
    ) {
        let Some(src) = src else {
            debug!(component = %canon, "lower: no source text; geometry not lowered");
            return;
        };
        let from = OpSource {
            file: src.file,
            src: src.text,
        };
        let g = component_geometry(&self.resolver, &component_id(canon), c);
        for d in &g.datums {
            self.lower_datum(canon, d, &from, comp);
        }
        if let Some(a) = &g.assembly {
            for p in &a.placements {
                self.lower_placement(canon, c, p, &from, comp);
            }
        }
        self.lower_points(canon, Some(src), c, comp);
    }

    fn lower_datum(
        &self,
        canon: &CanonPath,
        d: &DatumDef,
        from: &OpSource<'_>,
        comp: &mut Component,
    ) {
        let (Some(path), Some(parent), Some(ops)) = (
            path_of(&datum_path(canon, &d.name)),
            path_of(&datum_path(canon, &d.base)),
            ir_ops(&d.ops),
        ) else {
            return;
        };
        match datum_members(d, from) {
            Ok(ms) => self.push_members(ms, comp),
            Err(e) => {
                warn!(datum = %d.name, error = %e, "lower: datum members dropped");
                return;
            }
        }
        debug!(datum = %path, ops = ops.len(), "lower: datum");
        comp.datums.push(Datum::new(path, parent, ops));
    }

    /// Appends op members, filling each value from the evaluation of its canonical path.
    fn push_members(&self, ms: Vec<Member>, comp: &mut Component) {
        for mut m in ms {
            if let Ok(canon) = CanonPath::parse(&m.path.to_string()) {
                self.fill_geometry_value(&mut m, &canon);
            }
            comp.members.push(m);
        }
    }

    fn lower_placement(
        &self,
        canon: &CanonPath,
        c: &ast::Component,
        p: &GeoPlacement,
        from: &OpSource<'_>,
        comp: &mut Component,
    ) {
        let Some(part) = find_part(c, &p.node) else {
            return;
        };
        let counted = part.count.is_some();
        if counted && p.cover == Cover::Whole {
            let mut key = canon.clone();
            key.members.push(MemberSeg::Name(p.node.clone()));
            let Some(n) = self.input.evaluation.part_counts.get(&key).copied() else {
                warn!(node = %p.node, "lower: array placement skipped, count unknown");
                return;
            };
            for q in expand_instances(p, n) {
                self.lower_one_placement(canon, c, &q, from, comp);
            }
            return;
        }
        self.lower_one_placement(canon, c, p, from, comp);
    }

    fn lower_one_placement(
        &self,
        canon: &CanonPath,
        c: &ast::Component,
        p: &GeoPlacement,
        from: &OpSource<'_>,
        comp: &mut Component,
    ) {
        let Some(part) = find_part(c, &p.node) else {
            return;
        };
        let Some((ty, _)) = self
            .resolver
            .part_type(&canon.package, &canon.modules, &part.ty)
        else {
            warn!(node = %p.node, "lower: placed part type unresolved");
            return;
        };
        let index = match p.cover {
            Cover::Instance(i) => Some(Index::At(i)),
            Cover::Whole => None,
        };
        let mut part_path = canon.item_only();
        part_path.parts.push(PartSeg {
            name: p.node.clone(),
            index,
        });
        let mut place_path = part_path.clone();
        place_path.members.push(MemberSeg::Name("place".to_owned()));
        let at = match &p.at {
            Anchor::Origin => datum_path(canon, ORIGIN),
            Anchor::Datum { node, datum } => {
                let Some(other) = find_part(c, node) else {
                    return;
                };
                let Some((oty, _)) =
                    self.resolver
                        .part_type(&canon.package, &canon.modules, &other.ty)
                else {
                    warn!(node = %node, "lower: anchor part type unresolved");
                    return;
                };
                dotted_datum_path(&oty, datum)
            }
        };
        let (Some(path), Some(part_ir), Some(datum), Some(at), Some(ops)) = (
            path_of(&place_path),
            path_of(&part_path),
            path_of(&dotted_datum_path(&ty, &p.datum)),
            path_of(&at),
            ir_ops(&p.ops),
        ) else {
            return;
        };
        match op_members(p, from) {
            Ok(ms) => self.push_members(ms, comp),
            Err(e) => {
                warn!(node = %p.node, error = %e, "lower: placement members dropped");
                return;
            }
        }
        debug!(placement = %path, ops = ops.len(), "lower: placement");
        comp.placements.push(Placement {
            path,
            part: part_ir,
            datum,
            at,
            ops,
            variants: None,
            digest: None,
            closure: None,
        });
    }

    /// Points: typed point parameters and lets whose value is a point or an array of points.
    fn lower_points(
        &self,
        canon: &CanonPath,
        src: Option<&SourceText<'_>>,
        c: &ast::Component,
        comp: &mut Component,
    ) {
        let ev = self.input.evaluation;
        for m in &c.members {
            let (name, ty, expr) = match &m.kind {
                ast::MemberKind::Param(p) => (&p.name.name, p.ty.as_ref(), &p.value),
                ast::MemberKind::Let(l) => (&l.name.name, l.ty.as_ref(), &l.value),
                _ => continue,
            };
            let Some(pt) = ty.and_then(|t| point_type(unwrap_array(t))) else {
                continue;
            };
            let mut decl = canon.clone();
            decl.members.push(MemberSeg::Name(name.clone()));
            let Some(e) = ev.get(&decl) else {
                trace!(path = %decl, "lower: point member has no value");
                continue;
            };
            let text = text_of(src, expr.span);
            let items: Vec<(Option<u64>, &Val)> = match &e.value {
                Val::Point(_) => vec![(None, &e.value)],
                Val::Array(vs) if vs.iter().all(|v| matches!(v, Val::Point(_))) => {
                    (0u64..).zip(vs.iter()).map(|(i, v)| (Some(i), v)).collect()
                }
                other => {
                    trace!(path = %decl, kind = ?std::mem::discriminant(other), "lower: not a point value");
                    continue;
                }
            };
            let datum = dotted_datum_path(canon, &pt.datum.0);
            let Some(datum_ir) = path_of(&datum) else {
                continue;
            };
            for (i, _) in items {
                self.lower_point(&decl, name, i, &text, &datum_ir, comp);
            }
        }
    }

    fn lower_point(
        &self,
        decl: &CanonPath,
        name: &str,
        i: Option<u64>,
        text: &str,
        datum: &Path,
        comp: &mut Component,
    ) {
        let mut base = decl.clone();
        if let Some(i) = i {
            base.members.push(MemberSeg::Index(Index::At(i)));
        }
        let Some(path) = path_of(&base) else { return };
        let suffix = i.map(|i| format!("[{i}]")).unwrap_or_default();
        let mut coords = Vec::new();
        for axis in ["x", "y", "z"] {
            let mut cp = base.clone();
            cp.members.push(MemberSeg::Name(axis.to_owned()));
            let Some(coord_value) =
                self.coord_value(decl, i, axis, &cp, &format!("{name}{suffix}.{axis}"))
            else {
                break;
            };
            let Some(cpath) = path_of(&cp) else { break };
            let mut m = Member::new(cpath, MemberKind::Let);
            m.analysis = true;
            m.provenance = Provenance::Derived;
            m.expr = Some(text.to_owned());
            m.deps = path_of(decl).into_iter().collect();
            m.value = Some(coord_value.0.clone());
            m.variants = coord_value.1;
            coords.push(coord_value.0);
            comp.members.push(m);
        }
        debug!(point = %path, coords = coords.len(), "lower: point");
        comp.points.push(Point {
            path,
            datum: datum.clone(),
            coords,
        });
    }

    /// The value (and differing variant values) of one coordinate of a point member.
    #[allow(clippy::type_complexity)]
    fn coord_value(
        &self,
        decl: &CanonPath,
        i: Option<u64>,
        axis: &str,
        cp: &CanonPath,
        authored: &str,
    ) -> Option<(Value, Option<std::collections::BTreeMap<String, Value>>)> {
        let pick = |ev: &crate::eval::Evaluation| -> Option<crate::eval::Evaluated> {
            let e = ev.get(decl)?;
            let p = match (&e.value, i) {
                (Val::Point(_), None) => &e.value,
                (Val::Array(vs), Some(i)) => vs.get(usize::try_from(i).ok()?)?,
                _ => return None,
            };
            let Val::Point(cs) = p else { return None };
            let k = match axis {
                "x" => 0,
                "y" => 1,
                _ => 2,
            };
            let mut out = e.clone();
            out.value = cs.get(k)?.clone();
            out.var = None;
            out.alias_of = None;
            Some(out)
        };
        let ev = self.input.evaluation;
        let primary = pick(ev)?;
        let value = evaluated_value(ev, &self.owners, cp, &primary, authored)?;
        let mut variants = std::collections::BTreeMap::new();
        for (key, vev) in &self.input.variants {
            let Some(e) = pick(vev) else { continue };
            let owners = self.variant_owners.get(key).unwrap_or(&self.owners);
            if let Some(v) = evaluated_value(vev, owners, cp, &e, authored)
                && v != value
            {
                variants.insert(key.clone(), v);
            }
        }
        Some((value, (!variants.is_empty()).then_some(variants)))
    }
}
