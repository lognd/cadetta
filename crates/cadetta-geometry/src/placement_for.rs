//! For-placements over part arrays, `D.at(point)` and G007 (geometry.md 4).
//!
//! `place feet[i].top on base.underside.at(base.foot_sites[i]) for i in 0..count(feet)` places
//! every instance of a part array. [`lower_place_for`] splits the final `.at(point)` off the
//! anchor chain and lowers the rest with [`lower_place`]; [`ForPlacement::placement`] puts the
//! point back as one `.offset(x, y, z)` whose three arguments are members reading the point
//! expression, so a point is never split into two Lengths in the source. [`place_instances`]
//! evaluates the placements once per instance index, binding the range variable, and returns one
//! [`PlacedInstance`] per instance; [`check_unplaced`] raises G007.
//!
//! Decisions made here (written into geometry.md 4): `.at(point)` must be the last operation of
//! the anchor chain and takes exactly one positional argument; a `Point2` offsets x and y, a
//! `Point3` also z; instance `i` of a for-placement is evaluated with the range variable bound
//! to `i`, and instance paths read `Assembly/feet[i]`; G007 is judged per array node, only for
//! arrays that have at least one placement (an array with none is a G003 matter), and a `Whole`
//! placement covers every instance; a range running past the array count is not G007 (it is the
//! count check's concern).
// frob:ticket 01M4SWFXDRPA71KJBBBNGRNR86

use std::collections::BTreeSet;

use cadetta_sema::resolve::{CanonPath, MemberSeg};
use cadetta_syntax::ast::{Arg, Expr, ExprKind, Place};
use cadetta_syntax::span::Span;
use cadetta_units::{Affine, Dim, Dimension, Magnitude, Rational};
use tracing::{debug, trace, warn};

use crate::datum::{Axis, ComponentId, DatumOp, GeometryError, Member};
use crate::placement::{Assembly, Cover, Placement, lower_place, place_assembly};
use crate::transform::{Transform, TransformError};

/// The rule code for an array instance left without a placement.
pub const G007: &str = "G007";

/// The point written in `.at(point)`, still an expression.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct AtPoint {
    /// The point expression (`base.foot_sites[i]`).
    pub point: Expr,
}

/// A point evaluated for one instance: x and y, and z for a `Point3`.
#[derive(Clone, PartialEq, Debug)]
pub struct PointValue {
    /// The x coordinate.
    pub x: Affine,
    /// The y coordinate.
    pub y: Affine,
    /// The z coordinate of a `Point3`; `None` for a `Point2`.
    pub z: Option<Affine>,
}

/// A lowered `place` statement that may carry `.at(point)` and a `for` range.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ForPlacement {
    /// The placement, without the `.at` offset.
    pub base: Placement,
    /// The final `.at(point)`, if written.
    pub at: Option<AtPoint>,
    /// The `for` variable (`i`), if the statement has a range.
    pub range_var: Option<String>,
}

impl ForPlacement {
    /// The placement with `.at(point)` appended as an offset of x, y and z members.
    pub fn placement(&self) -> Placement {
        let mut p = self.base.clone();
        if let Some(at) = &self.at {
            let n = p.ops.len();
            let args = [Axis::X, Axis::Y, Axis::Z]
                .into_iter()
                .map(|axis| {
                    let mut path = p
                        .members()
                        .first()
                        .map(|m| m.path.clone())
                        .unwrap_or_else(|| CanonPath::item("", &[], ""));
                    path.members.clear();
                    path.members.push(MemberSeg::Name("place".into()));
                    path.members.push(MemberSeg::Name(n.to_string()));
                    path.members.push(MemberSeg::Name(axis.name().into()));
                    (
                        axis,
                        Member {
                            path,
                            value: at.point.clone(),
                        },
                    )
                })
                .collect();
            p.ops.push(DatumOp::Offset { args });
        }
        p
    }
}

fn length_dim() -> Dim {
    Dim::new(Dimension::from_si([1, 0, 0, 0, 0, 0, 0]), 0)
}

fn zero_length() -> Affine {
    Affine::constant(length_dim(), Magnitude::new(Rational::from_int(0), 0))
}

fn is_at(callee: &Expr) -> Option<Expr> {
    match &callee.kind {
        ExprKind::Field(inner, name) if name.name == "at" => Some((**inner).clone()),
        _ => None,
    }
}

/// Splits a final `.at(point)` off `on`: the chain without it, and the point.
///
/// `Ok(None)` when `on` has no `.at`; an error when `.at` is not the last operation or does not
/// take exactly one positional argument.
pub fn split_at(on: &Expr) -> Result<Option<(Expr, AtPoint)>, GeometryError> {
    fn mentions_at(e: &Expr) -> bool {
        match &e.kind {
            ExprKind::Paren(i) => mentions_at(i),
            ExprKind::Field(i, n) => n.name == "at" || mentions_at(i),
            ExprKind::Call { callee, .. } => mentions_at(callee),
            ExprKind::Path(p) => p.segments.iter().skip(1).any(|s| s.name == "at"),
            _ => false,
        }
    }
    let bad = |span: Span, m: &str| {
        warn!(message = m, "at(point)");
        Err(GeometryError {
            span,
            message: m.to_owned(),
        })
    };
    match &on.kind {
        ExprKind::Paren(inner) => split_at(inner),
        ExprKind::Call {
            callee,
            optional: false,
            args,
        } => {
            if let Some(inner) = is_at(callee) {
                let [
                    Arg {
                        name: None, value, ..
                    },
                ] = args.as_slice()
                else {
                    return bad(on.span, "`.at(point)` takes one positional point");
                };
                if mentions_at(&inner) {
                    return bad(on.span, "`.at(point)` must be the last operation");
                }
                debug!("split .at(point) off the anchor chain");
                return Ok(Some((
                    inner,
                    AtPoint {
                        point: value.clone(),
                    },
                )));
            }
            if mentions_at(callee) {
                return bad(on.span, "`.at(point)` must be the last operation");
            }
            Ok(None)
        }
        _ => Ok(None),
    }
}

/// Lowers `place target on expr[.at(point)] [for i in lo..hi]`.
///
/// `index` evaluates a literal or `count(..)` bound to an instance number, as for
/// [`lower_place`].
pub fn lower_place_for(
    id: &ComponentId,
    p: &Place,
    index: &dyn Fn(&Expr) -> Option<u64>,
) -> Result<ForPlacement, GeometryError> {
    let (on, at) = match split_at(&p.on)? {
        Some((rest, at)) => (rest, Some(at)),
        None => (p.on.clone(), None),
    };
    let plain = Place { on, ..p.clone() };
    let base = lower_place(id, &plain, index)?;
    debug!(node = %base.node, at = at.is_some(), "lowered for-placement");
    Ok(ForPlacement {
        base,
        at,
        range_var: p.each.as_ref().map(|r| r.var.name.clone()),
    })
}

/// Offsets `t` to the point's x and y (and z for a `Point3`), in the current frame.
pub fn offset_to_point(t: &Transform, p: &PointValue) -> Result<Transform, TransformError> {
    t.offset(&[Some(p.x.clone()), Some(p.y.clone()), p.z.clone()])
}

/// One placed instance of a part array.
#[derive(Clone, PartialEq, Debug)]
pub struct PlacedInstance {
    /// The instance path, e.g. `Mount/feet[2]`.
    pub path: String,
    /// The instance index.
    pub index: u64,
    /// The instance's origin frame in the root assembly's frame.
    pub transform: Transform,
}

/// Evaluates every instance of every ranged placement in `a`.
///
/// `a.placements` must hold [`ForPlacement::placement`] results; `fors` names the `.at` points so
/// their synthetic members are read through `point` (the point of instance `i`) rather than
/// `eval`. `eval(member, i)` evaluates an ordinary member with the range variable bound to `i`.
pub fn place_instances(
    a: &Assembly,
    fors: &[ForPlacement],
    eval: &dyn Fn(&Member, u64) -> Result<Affine, GeometryError>,
    point: &dyn Fn(&Expr, u64) -> Result<PointValue, GeometryError>,
) -> Result<Vec<PlacedInstance>, GeometryError> {
    let at_spans: BTreeSet<Span> = fors
        .iter()
        .filter_map(|f| f.at.as_ref().map(|at| at.point.span))
        .collect();
    let top = a
        .placements
        .iter()
        .filter_map(|p| match p.cover {
            Cover::Range { hi, .. } => Some(hi),
            Cover::Whole => None,
        })
        .max()
        .unwrap_or(0);
    let mut out = Vec::new();
    for i in 0..top {
        let member = |m: &Member| -> Result<Affine, GeometryError> {
            if !at_spans.contains(&m.value.span) {
                return eval(m, i);
            }
            let v = point(&m.value, i)?;
            match m.path.members.last() {
                Some(MemberSeg::Name(n)) if n == "x" => Ok(v.x),
                Some(MemberSeg::Name(n)) if n == "y" => Ok(v.y),
                _ => Ok(v.z.unwrap_or_else(zero_length)),
            }
        };
        for part in place_assembly(a, &member)? {
            if let Cover::Range { lo, hi } = part.cover
                && (lo..hi).contains(&i)
            {
                trace!(path = %part.path, index = i, "instance placed");
                out.push(PlacedInstance {
                    path: format!("{}[{i}]", part.path),
                    index: i,
                    transform: part.transform,
                });
            }
        }
    }
    debug!(instances = out.len(), "placed instances");
    Ok(out)
}

/// A G007 diagnostic.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct UnplacedDiag {
    /// Always [`G007`].
    pub code: &'static str,
    /// The first placement of the array.
    pub span: Span,
    /// The instances left without a placement.
    pub message: String,
}

/// Raises G007 for each placed array of `a` (and its subassemblies) that has unplaced instances.
///
/// `count` gives the instance count of the array node at an instance path such as `Mount/feet`,
/// and `None` for a scalar part.
pub fn check_unplaced(a: &Assembly, count: &dyn Fn(&str) -> Option<u64>) -> Vec<UnplacedDiag> {
    let mut out = Vec::new();
    check_into(a, &a.name, count, &mut out);
    out
}

fn check_into(
    a: &Assembly,
    prefix: &str,
    count: &dyn Fn(&str) -> Option<u64>,
    out: &mut Vec<UnplacedDiag>,
) {
    for node in &a.nodes {
        let path = format!("{prefix}/{}", node.name);
        if let Some(sub) = &node.sub {
            check_into(sub, &path, count, out);
        }
        let Some(n) = count(&path) else { continue };
        let own: Vec<&Placement> = a
            .placements
            .iter()
            .filter(|p| p.node == node.name)
            .collect();
        let Some(first) = own.first() else { continue };
        if own.iter().any(|p| p.cover == Cover::Whole) {
            continue;
        }
        let missing: Vec<u64> = (0..n)
            .filter(|i| {
                !own.iter().any(|p| match p.cover {
                    Cover::Range { lo, hi } => (lo..hi).contains(i),
                    Cover::Whole => true,
                })
            })
            .collect();
        if missing.is_empty() {
            continue;
        }
        let list = missing
            .iter()
            .map(|i| format!("{}[{i}]", node.name))
            .collect::<Vec<_>>()
            .join(", ");
        let message = format!("{list} of `{path}` has no placement");
        warn!(code = G007, %message, "unplaced instance");
        out.push(UnplacedDiag {
            code: G007,
            span: first.span,
            message,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::datum::lower_component_datums;
    use crate::placement::Node;
    use cadetta_syntax::ast::{Component, ComponentClass, FileEntry, ItemKind, MemberKind};
    use cadetta_syntax::parser::parse;

    fn id(item: &str) -> ComponentId {
        ComponentId {
            package: "acme.rover".into(),
            modules: vec![],
            item: item.into(),
        }
    }

    fn mm(n: i64) -> Affine {
        Affine::constant(
            length_dim(),
            Magnitude::new(Rational::new(n, 1000).unwrap(), 0),
        )
    }

    fn center(c: &crate::transform::Coord) -> Magnitude {
        match c {
            crate::transform::Coord::Exact(a) => a.center().clone(),
            _ => panic!("inexact"),
        }
    }

    fn count(e: &Expr) -> Option<u64> {
        match &e.kind {
            ExprKind::Number(n) => n.parse().ok(),
            ExprKind::Paren(i) => count(i),
            _ => Some(4),
        }
    }

    const MODELS: &str = "model Base {\n  datum underside = origin.offset(z = -2mm)\n}\nmodel Foot {\n  datum top = origin.offset(z = 3mm)\n}\n";

    fn comps(src: &str) -> Vec<Component> {
        let parsed = parse(src);
        assert!(parsed.diags.is_empty(), "{:?}", parsed.diags);
        parsed
            .file
            .entries
            .iter()
            .filter_map(|e| match e {
                FileEntry::Item(i) => match &i.kind {
                    ItemKind::Component(c) => Some(c.clone()),
                    _ => None,
                },
                _ => None,
            })
            .collect()
    }

    fn build(asm: &str) -> (Assembly, Vec<ForPlacement>) {
        let all = comps(&format!("{MODELS}{asm}"));
        let a = all
            .iter()
            .find(|c| c.class == ComponentClass::Assembly)
            .unwrap();
        let mut nodes = Vec::new();
        for m in &a.members {
            if let MemberKind::Part(p) = &m.kind {
                let ty = match &p.ty.kind {
                    cadetta_syntax::ast::TypeKind::Path(path) => path.text(),
                    _ => panic!("part type"),
                };
                let datums = all
                    .iter()
                    .find(|c| c.name.name == ty)
                    .map(|c| lower_component_datums(&id(&c.name.name), c).0)
                    .unwrap_or_default();
                nodes.push(Node {
                    name: p.name.name.clone(),
                    datums,
                    sub: None,
                });
            }
        }
        let cid = id(&a.name.name);
        let fors: Vec<ForPlacement> = a
            .members
            .iter()
            .filter_map(|m| match &m.kind {
                MemberKind::Place(p) => Some(lower_place_for(&cid, p, &count).unwrap()),
                _ => None,
            })
            .collect();
        let asm = Assembly {
            name: a.name.name.clone(),
            nodes,
            placements: fors.iter().map(ForPlacement::placement).collect(),
        };
        (asm, fors)
    }

    fn mm_of(e: &Expr) -> Option<Affine> {
        match &e.kind {
            ExprKind::Quantity(s) => Some(mm(s[0].number.parse().ok()?)),
            ExprKind::Paren(i) => mm_of(i),
            ExprKind::Unary(_, i) => mm_of(i).map(|a| a.neg()),
            _ => None,
        }
    }

    fn eval(m: &Member, _: u64) -> Result<Affine, GeometryError> {
        mm_of(&m.value).ok_or(GeometryError {
            span: m.value.span,
            message: "test eval".into(),
        })
    }

    fn point(_: &Expr, i: u64) -> Result<PointValue, GeometryError> {
        Ok(PointValue {
            x: mm(10 * i as i64),
            y: mm(5),
            z: None,
        })
    }

    const ROW: &str = "assembly Mount {\n  part base: Base\n  part feet[4]: Foot\n  place base.underside on origin\n  place feet[i].top on base.underside.at(base.foot_sites[i]) for i in 0..count(feet)\n}\n";

    // frob:tests crates/cadetta-geometry/src/placement_for.rs::place_instances kind=unit
    #[test]
    fn every_instance_is_placed() {
        let (a, fors) = build(ROW);
        assert!(check_unplaced(&a, &|p| (p == "Mount/feet").then_some(4)).is_empty());
        let placed = place_instances(&a, &fors, &eval, &point).unwrap();
        assert_eq!(placed.len(), 4);
        assert_eq!(placed[3].path, "Mount/feet[3]");
        assert_eq!(placed[3].index, 3);
    }

    // frob:tests crates/cadetta-geometry/src/placement_for.rs::lower_place_for kind=unit
    #[test]
    fn at_offsets_to_the_point() {
        let (a, fors) = build(ROW);
        assert!(fors[1].at.is_some());
        assert_eq!(fors[1].range_var.as_deref(), Some("i"));
        let placed = place_instances(&a, &fors, &eval, &point).unwrap();
        let t = &placed[2].transform;
        // the underside is placed on the origin; point x = 20mm, y = 5mm; foot top is 3mm above its origin.
        assert_eq!(center(t.trans(0)), center_of(mm(20)));
        assert_eq!(center(t.trans(1)), center_of(mm(5)));
        assert_eq!(center(t.trans(2)), center_of(mm(-3)));
    }

    fn center_of(a: Affine) -> Magnitude {
        a.center().clone()
    }

    // frob:tests crates/cadetta-geometry/src/placement_for.rs::offset_to_point kind=unit
    #[test]
    fn point3_sets_z() {
        let t = Transform::identity(length_dim());
        let p = PointValue {
            x: mm(1),
            y: mm(2),
            z: Some(mm(3)),
        };
        let t = offset_to_point(&t, &p).unwrap();
        assert_eq!(center(t.trans(2)), center_of(mm(3)));
    }

    // frob:tests crates/cadetta-geometry/src/placement_for.rs::split_at kind=unit
    #[test]
    fn at_must_be_last() {
        let all = comps(&format!(
            "{MODELS}assembly M {{\n  part base: Base\n  part feet[4]: Foot\n  place feet[i].top on base.underside.at(base.s[i]).offset(z = 1mm) for i in 0..4\n}}\n"
        ));
        let a = all.last().unwrap();
        let Some(MemberKind::Place(p)) = a
            .members
            .iter()
            .map(|m| &m.kind)
            .find(|k| matches!(k, MemberKind::Place(_)))
        else {
            panic!()
        };
        assert!(split_at(&p.on).is_err());
    }

    // frob:tests crates/cadetta-geometry/src/placement_for.rs::check_unplaced kind=unit
    #[test]
    fn g007_on_unplaced_instance() {
        let (a, _) = build(
            "assembly Mount {\n  part base: Base\n  part feet[4]: Foot\n  place base.underside on origin\n  place feet[i].top on base.underside.at(base.s[i]) for i in 0..2\n}\n",
        );
        let d = check_unplaced(&a, &|p| (p == "Mount/feet").then_some(4));
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].code, "G007");
        assert!(
            d[0].message.contains("feet[2], feet[3]"),
            "{}",
            d[0].message
        );
    }

    // frob:tests crates/cadetta-geometry/src/placement_for.rs::check_unplaced kind=unit
    #[test]
    fn g007_must_not_fire_for_split_ranges() {
        let (a, _) = build(
            "assembly Mount {\n  part base: Base\n  part feet[4]: Foot\n  place base.underside on origin\n  place feet[i].top on base.underside for i in 0..2\n  place feet[i].top on base.underside for i in 2..4\n}\n",
        );
        assert!(check_unplaced(&a, &|p| (p == "Mount/feet").then_some(4)).is_empty());
    }
}
