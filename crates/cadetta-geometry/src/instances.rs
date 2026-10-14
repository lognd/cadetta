//! Elementwise array placement, per-instance lowering, G004 and G007 (geometry.md 4).
//!
//! `place feet.top at base.underside.offset(base.foot_sites)` places instance i of `feet` at
//! point i of `base.foot_sites`. [`check_elementwise`] raises G004 when the array operands of
//! the anchor do not match the part array (count mismatch, no array operand, or an array operand
//! on an indexed target). [`expand_instances`] lowers an array placement to one placement per
//! instance with integer indexes, so the IR never carries `[*]` (ir.md 2). [`place_instances`]
//! evaluates every instance with every array operand indexed by i; [`check_unplaced`] raises
//! G007 for an array placed instance by instance with instances left over.
//!
//! Decisions made here (written into geometry.md 4): a point `Point2` offsets x and y, a
//! `Point3` also z; instance paths read `Assembly/feet[i]`; G007 is judged per array node that
//! has at least one indexed placement and no array placement, and lists the missing instances
//! in 0 to count - 1; a scalar target with an array operand is not judged here.
// frob:ticket 01M4VB636BKBDGRW3NBMGK7CBZ

use cadetta_ir::canon::Index;
use cadetta_syntax::ast::Expr;
use cadetta_syntax::span::Span;
use cadetta_units::Affine;
use tracing::{debug, trace, warn};

use crate::datum::{Angle, DatumOp, GeometryError, Member};
use crate::placement::{Anchor, Assembly, Cover, Placement, PlacementCode, PlacementDiag};
use crate::placement::{PlacedInstance, place_instance_frame};
use crate::transform::{Transform, TransformError};

/// The rule code for an array instance left without a placement.
pub const G007: &str = "G007";

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

/// Offsets `t` to the point's x and y (and z for a `Point3`), in the current frame.
pub fn offset_to_point(t: &Transform, p: &PointValue) -> Result<Transform, TransformError> {
    t.offset(&[Some(p.x.clone()), Some(p.y.clone()), p.z.clone()])
}

/// An array-valued operand of a placement anchor.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Operand {
    /// The point of the point form `D.offset(p)`.
    Point(Expr),
    /// The datum of another node of the assembly (an array when that node is a part array).
    Datum(String),
}

/// The operands of `p`'s anchor that may be arrays, in source order.
pub fn operands(p: &Placement) -> Vec<Operand> {
    let mut out = Vec::new();
    if let Anchor::Datum { node, .. } = &p.at {
        out.push(Operand::Datum(node.clone()));
    }
    if let Some(pt) = &p.point {
        out.push(Operand::Point(pt.clone()));
    }
    out
}

/// G004 for one placement.
///
/// `target_count` is the instance count when the target node is a part array. `len` gives the
/// length of an array operand and `None` for a single-valued one (an indexed point such as
/// `sites[2]` is single-valued).
pub fn check_elementwise(
    p: &Placement,
    target_count: Option<u64>,
    len: &dyn Fn(&Operand) -> Option<u64>,
) -> Vec<PlacementDiag> {
    let mut out = Vec::new();
    let mut bad = |msg: String| {
        warn!(code = "G004", message = %msg, "elementwise placement");
        out.push(PlacementDiag {
            code: PlacementCode::G004,
            span: p.span,
            message: msg,
        });
    };
    let lens: Vec<(Operand, Option<u64>)> = operands(p)
        .into_iter()
        .map(|o| {
            let l = len(&o);
            (o, l)
        })
        .collect();
    match (p.cover, target_count) {
        (Cover::Instance(_), _) => {
            if lens.iter().any(|(_, l)| l.is_some()) {
                bad(format!(
                    "`{}` is placed instance by instance, so its anchor must be single-valued",
                    p.node
                ));
            }
        }
        (Cover::Whole, Some(n)) => {
            if lens.iter().all(|(_, l)| l.is_none()) {
                bad(format!(
                    "`{}` is an array of {n} placed with an anchor that has no array operand; every instance would share one frame",
                    p.node
                ));
            }
            for (_, l) in &lens {
                if let Some(l) = l
                    && *l != n
                {
                    bad(format!(
                        "`{}` has {n} instances but an anchor operand has {l}",
                        p.node
                    ));
                }
            }
        }
        (Cover::Whole, None) => {}
    }
    out
}

/// Lowers an array placement to one placement per instance with integer indexes.
///
/// Every Member keeps its expression and span and takes the path `Assembly/part[i].place...`.
/// An indexed placement is returned unchanged.
pub fn expand_instances(p: &Placement, count: u64) -> Vec<Placement> {
    if p.cover != Cover::Whole {
        return vec![p.clone()];
    }
    (0..count)
        .map(|i| {
            let mut q = p.clone();
            q.cover = Cover::Instance(i);
            let fix = |m: &mut Member| {
                if let Some(seg) = m.path.parts.iter_mut().rev().find(|s| s.name == p.node) {
                    seg.index = Some(Index::At(i));
                }
            };
            for op in q.ops.iter_mut() {
                match op {
                    DatumOp::Offset { args } => args.iter_mut().for_each(|(_, m)| fix(m)),
                    DatumOp::Rotate {
                        angle: Angle::Given(m),
                        ..
                    } => fix(m),
                    DatumOp::Rotate { .. } => {}
                }
            }
            trace!(node = %p.node, index = i, "expanded array placement");
            q
        })
        .collect()
}

/// Evaluates every instance of every part array in `a` that `count` knows.
///
/// `count(name)` is the instance count of the array node `name` (`None` for a scalar part).
/// `eval(member, i)` evaluates an ordinary member for instance `i`; the point Members of the
/// point form are read through `point(expr, i)`, the point of instance `i`.
pub fn place_instances(
    a: &Assembly,
    count: &dyn Fn(&str) -> Option<u64>,
    eval: &dyn Fn(&Member, u64) -> Result<Affine, GeometryError>,
    point: &dyn Fn(&Expr, u64) -> Result<PointValue, GeometryError>,
) -> Result<Vec<PlacedInstance>, GeometryError> {
    let point_spans: Vec<Span> = a
        .placements
        .iter()
        .filter_map(|p| p.point.as_ref().map(|e| e.span))
        .collect();
    let mut out = Vec::new();
    for node in &a.nodes {
        let Some(n) = count(&node.name) else { continue };
        for i in 0..n {
            let member = |m: &Member| -> Result<Affine, GeometryError> {
                if !point_spans.contains(&m.value.span) {
                    return eval(m, i);
                }
                let v = point(&m.value, i)?;
                match m.path.members.last() {
                    Some(cadetta_ir::canon::MemberSeg::Name(n)) if n == "x" => Ok(v.x),
                    Some(cadetta_ir::canon::MemberSeg::Name(n)) if n == "y" => Ok(v.y),
                    _ => v.z.ok_or(GeometryError {
                        span: m.value.span,
                        message: "a Point2 has no z".to_owned(),
                    }),
                }
            };
            if let Some(t) = place_instance_frame(a, &node.name, i, &member)? {
                trace!(node = %node.name, index = i, "instance placed");
                out.push(PlacedInstance {
                    path: format!("{}/{}[{i}]", a.name, node.name),
                    index: i,
                    transform: t,
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

/// Raises G007 for each array of `a` (and its subassemblies) placed instance by instance with
/// instances left over.
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
            .filter(|i| !own.iter().any(|p| p.cover.covers(Some(*i))))
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
    use crate::placement::tests::{build_with, eval, src};
    use crate::placement::{PointInfo, check_assembly, length_dim, lower_place};
    use cadetta_units::{Magnitude, Rational};

    fn mm(n: i64) -> Affine {
        Affine::constant(
            length_dim(),
            Magnitude::new(Rational::new(n, 1000).unwrap(), 0),
        )
    }

    fn sites(e: &Expr) -> Option<PointInfo> {
        match &e.kind {
            cadetta_syntax::ast::ExprKind::Path(p) if p.text() == "sites" => Some(PointInfo {
                datum: "top".to_owned(),
                arity: 2,
            }),
            cadetta_syntax::ast::ExprKind::Path(p) if p.text() == "other" => Some(PointInfo {
                datum: "site".to_owned(),
                arity: 2,
            }),
            _ => None,
        }
    }

    fn feet(place: &str) -> Assembly {
        build_with(
            &src(&format!(
                "assembly Mount {{\n  part rail: Rail\n  part feet[3]: Bracket\n  place rail.origin at origin\n  {place}\n}}\n"
            )),
            &[],
            &sites,
        )
    }

    fn point_i(_: &Expr, i: u64) -> Result<PointValue, GeometryError> {
        Ok(PointValue {
            x: mm(10 * i64::try_from(i).unwrap()),
            y: mm(0),
            z: None,
        })
    }

    fn only_feet(n: &str) -> Option<u64> {
        (n == "feet").then_some(3)
    }

    // frob:tests crates/cadetta-geometry/src/instances.rs::place_instances kind=unit
    #[test]
    fn elementwise_placement_puts_instance_i_at_point_i() {
        let a = feet("place feet.base at rail.top.offset(sites)");
        assert!(check_assembly(&a).is_empty());
        assert!(a.placements[1].point.is_some());
        let placed = place_instances(&a, &only_feet, &|m, _| eval(m), &point_i).unwrap();
        assert_eq!(placed.len(), 3);
        for (i, p) in placed.iter().enumerate() {
            assert_eq!(p.path, format!("Mount/feet[{i}]"));
            let want = Magnitude::new(
                Rational::new(10 * i64::try_from(i).unwrap(), 1000).unwrap(),
                0,
            );
            match p.transform.trans(0) {
                crate::transform::Coord::Exact(a) => assert_eq!(a.center(), &want),
                _ => panic!("inexact"),
            }
        }
    }

    // frob:tests crates/cadetta-geometry/src/placement.rs::lower_place kind=unit
    #[test]
    fn point_form_equals_the_explicit_axes() {
        let a = feet("place feet.base at rail.top.offset(sites)");
        let b = feet("place feet.base at rail.top.offset(x = 7mm, y = 7mm)");
        let paths = |a: &Assembly| -> Vec<String> {
            a.placements[1]
                .members()
                .iter()
                .map(|m| m.path.to_string())
                .collect()
        };
        assert_eq!(paths(&a), paths(&b));
        assert_eq!(
            paths(&a),
            [
                "acme.rover::Mount/feet.place.offset.x",
                "acme.rover::Mount/feet.place.offset.y"
            ]
        );
        // The Members' expression is the point.
        let pt = a.placements[1].point.clone().unwrap();
        assert!(
            a.placements[1]
                .members()
                .iter()
                .all(|m| m.value.span == pt.span)
        );
    }

    fn lower_err(place: &str) -> String {
        let s = src(&format!(
            "assembly Mount {{\n  part rail: Rail\n  part feet[3]: Bracket\n  {place}\n}}\n"
        ));
        let parsed = cadetta_syntax::parser::parse(&s);
        assert!(parsed.diags.is_empty(), "{:?}", parsed.diags);
        let id = crate::datum::ComponentId {
            package: "acme.rover".into(),
            modules: vec![],
            item: "Mount".into(),
        };
        for e in &parsed.file.entries {
            let cadetta_syntax::ast::FileEntry::Item(i) = e else {
                continue;
            };
            let cadetta_syntax::ast::ItemKind::Component(c) = &i.kind else {
                continue;
            };
            for m in &c.members {
                if let cadetta_syntax::ast::MemberKind::Place(p) = &m.kind {
                    return lower_place(&id, p, &|_| None, &sites).unwrap_err().message;
                }
            }
        }
        panic!("no placement");
    }

    // frob:tests crates/cadetta-geometry/src/placement.rs::lower_place kind=unit
    #[test]
    fn point_on_another_datum_or_after_an_op_is_g001() {
        let m = lower_err("place feet.base at rail.top.offset(other)");
        assert!(m.starts_with("G001"), "{m}");
        let m = lower_err("place feet.base at rail.top.offset(z = 1mm).offset(sites)");
        assert!(m.starts_with("G001"), "{m}");
    }

    // frob:tests crates/cadetta-geometry/src/instances.rs::check_elementwise kind=unit
    #[test]
    fn g004_count_mismatch_no_array_operand_and_indexed_target() {
        let a = feet("place feet.base at rail.top.offset(sites)");
        let p = &a.placements[1];
        let arr = |n: u64| move |o: &Operand| matches!(o, Operand::Point(_)).then_some(n);
        assert!(check_elementwise(p, Some(3), &arr(3)).is_empty());
        let d = check_elementwise(p, Some(3), &arr(4));
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].code, PlacementCode::G004);
        let none = |_: &Operand| None;
        assert_eq!(check_elementwise(p, Some(3), &none).len(), 1);
        let idx = feet("place feet[1].base at rail.top.offset(sites)");
        let d = check_elementwise(&idx.placements[1], Some(3), &arr(3));
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].code, PlacementCode::G004);
        assert!(check_elementwise(&idx.placements[1], Some(3), &none).is_empty());
    }

    // frob:tests crates/cadetta-geometry/src/instances.rs::expand_instances kind=unit
    #[test]
    fn expansion_gives_integer_indexed_instances() {
        let a = feet("place feet.base at rail.top.offset(x = 1mm)");
        let ex = expand_instances(&a.placements[1], 3);
        assert_eq!(ex.len(), 3);
        for (i, q) in ex.iter().enumerate() {
            assert_eq!(q.cover, Cover::Instance(u64::try_from(i).unwrap()));
            assert_eq!(q.span, a.placements[1].span);
        }
    }

    // frob:tests crates/cadetta-geometry/src/instances.rs::check_unplaced kind=unit
    #[test]
    fn g007_partly_placed_array() {
        let part = feet("place feet[0].base at rail.top\n  place feet[1].base at rail.top");
        let d = check_unplaced(&part, &|p| (p == "Mount/feet").then_some(3));
        assert_eq!(d.len(), 1, "{d:?}");
        assert_eq!(d[0].code, G007);
        assert!(d[0].message.contains("feet[2]"), "{}", d[0].message);
        let all = feet(
            "place feet[0].base at rail.top\n  place feet[1].base at rail.top\n  place feet[2].base at rail.top",
        );
        assert!(check_unplaced(&all, &|p| (p == "Mount/feet").then_some(3)).is_empty());
        let whole = feet("place feet.base at rail.top");
        assert!(check_unplaced(&whole, &|p| (p == "Mount/feet").then_some(3)).is_empty());
    }
}
