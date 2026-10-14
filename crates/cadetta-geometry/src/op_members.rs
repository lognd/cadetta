//! Placement op arguments as IR members (geometry.md 4; ir.md 2).
//!
//! [`lower_place`](crate::placement::lower_place) leaves each operation argument as a syntax-level
//! [`Member`] with a sema [`CanonPath`]. This module turns them into IR [`IrMember`]s: path
//! `Assembly/part.place.offset.x`, `place.offset[1].x`, `place.rotate.z` (ops numbered like datum
//! ops), kind `Place`, provenance `authored`, so bindings, R001, waivers and `why` attach to them.
//!
//! An array placement lowers per instance with integer indexes,
//! `Assembly/feet[2].place.offset.x` ([`instance_members`]); the Members of every instance share
//! the source expression and span. `[*]` is valid only in bindings, never in the IR (ir.md 2), so
//! [`ir_path`] rejects it.
// frob:ticket 01M4VB636BKBDGRW3NBMGK7CBZ

use cadetta_ir::canon::{CanonPath, Index, MemberSeg};
use cadetta_ir::model::{Member as IrMember, MemberKind, Provenance, SpanRef};
use cadetta_ir::path::{Index as IrIndex, Path as IrPath, Seg, Step};
use tracing::{debug, warn};

use crate::datum::{GeometryError, Member};
use crate::placement::Placement;

/// Where the placement was written: enough to fill an IR member's expression and span.
#[derive(Clone, Copy, Debug)]
pub struct OpSource<'a> {
    /// The file name recorded in each member's span.
    pub file: &'a str,
    /// The file text, from which the argument expression is read back.
    pub src: &'a str,
}

/// Converts a sema canonical path to an IR path; `[*]` is an error (IR indexes are integers).
pub fn ir_path(p: &CanonPath) -> Result<IrPath, String> {
    let index = |i: &Index| -> Result<IrIndex, String> {
        match i {
            Index::At(n) => Ok(IrIndex::Int(*n)),
            Index::Any => Err(format!("`{p}` has `[*]`; IR indexes are integers")),
        }
    };
    let steps = p
        .parts
        .iter()
        .map(|s| {
            Ok(Step {
                name: s.name.clone(),
                index: s.index.as_ref().map(index).transpose()?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let mut members = Vec::new();
    for m in &p.members {
        members.push(match m {
            MemberSeg::Name(n) => match n.parse::<u64>() {
                Ok(k) => Seg::Num(k),
                Err(_) => Seg::Name(n.clone()),
            },
            MemberSeg::Index(i) => Seg::Index(index(i)?),
        });
    }
    Ok(IrPath {
        package: p.package.clone(),
        modules: p.modules.clone(),
        item: p.item.clone(),
        steps,
        members,
    })
}

pub(crate) fn lower_member(
    m: &Member,
    kind: MemberKind,
    from: &OpSource<'_>,
) -> Result<IrMember, GeometryError> {
    let span = m.value.span;
    let path = ir_path(&m.path).map_err(|message| {
        warn!(%message, "placement member path");
        GeometryError { span, message }
    })?;
    let mut out = IrMember::new(path, kind);
    out.provenance = Provenance::Authored;
    out.expr = Some(span.slice(from.src).to_owned());
    out.span = Some(SpanRef {
        file: from.file.to_owned(),
        start: u32::try_from(span.start).unwrap_or(u32::MAX),
        end: u32::try_from(span.end).unwrap_or(u32::MAX),
    });
    debug!(path = %out.path, "placement op member");
    Ok(out)
}

/// Every operation argument of a datum declaration as an authored `Datum` IR member.
// frob:ticket 01M4VQXPDA4TEKE455FNYB5TXC
pub fn datum_members(
    d: &crate::datum::DatumDef,
    from: &OpSource<'_>,
) -> Result<Vec<IrMember>, GeometryError> {
    d.members()
        .into_iter()
        .map(|m| lower_member(m, MemberKind::Datum, from))
        .collect()
}

/// Every operation argument of `p` as an authored IR member, in op order.
pub fn op_members(p: &Placement, from: &OpSource<'_>) -> Result<Vec<IrMember>, GeometryError> {
    p.members()
        .into_iter()
        .map(|m| lower_member(m, MemberKind::Place, from))
        .collect()
}

/// Every operation argument of an array placement, once per instance (`feet[0]` .. `feet[n-1]`).
///
/// An indexed placement lowers to its own members unchanged.
pub fn instance_members(
    p: &Placement,
    count: u64,
    from: &OpSource<'_>,
) -> Result<Vec<IrMember>, GeometryError> {
    let mut out = Vec::new();
    for q in crate::instances::expand_instances(p, count) {
        out.extend(op_members(&q, from)?);
    }
    debug!(node = %p.node, members = out.len(), "array placement members");
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadetta_syntax::ast::{Expr, ExprKind};
    use cadetta_syntax::ast::{FileEntry, ItemKind, MemberKind as AstMember};
    use cadetta_syntax::parser::parse;

    use crate::datum::ComponentId;
    use crate::placement::lower_place;

    const SRC: &str = "model Bracket {\n  datum base = origin.offset(z = 5mm)\n}\nassembly Drive {\n  part plate: Bracket\n  part feet: [Bracket; 4]\n  place plate.base at origin.offset(x = 4mm).rotate(z = 90deg).offset(y = 1mm)\n  place feet.base at origin.offset(x = 9mm)\n}\n";

    fn count(e: &Expr) -> Option<u64> {
        match &e.kind {
            ExprKind::Number(n) => n.parse().ok(),
            _ => None,
        }
    }

    fn lowered() -> Vec<Placement> {
        let parsed = parse(SRC);
        assert!(parsed.diags.is_empty(), "{:?}", parsed.diags);
        let id = ComponentId {
            package: "acme.rover".into(),
            modules: vec![],
            item: "Drive".into(),
        };
        let mut out = Vec::new();
        for e in &parsed.file.entries {
            let FileEntry::Item(i) = e else { continue };
            let ItemKind::Component(c) = &i.kind else {
                continue;
            };
            for m in &c.members {
                if let AstMember::Place(p) = &m.kind {
                    out.push(lower_place(&id, p, &count, &|_| None).unwrap());
                }
            }
        }
        out
    }

    fn from() -> OpSource<'static> {
        OpSource {
            file: "drive.cdt",
            src: SRC,
        }
    }

    // frob:tests crates/cadetta-geometry/src/op_members.rs::op_members kind=unit
    #[test]
    fn placement_args_are_authored_place_members() {
        let all = lowered();
        let ms = op_members(&all[0], &from()).unwrap();
        let paths: Vec<String> = ms.iter().map(|m| m.path.to_string()).collect();
        assert_eq!(
            paths,
            [
                "acme.rover::Drive/plate.place.offset.x",
                "acme.rover::Drive/plate.place.rotate.z",
                "acme.rover::Drive/plate.place.offset[1].y",
            ]
        );
        for m in &ms {
            assert_eq!(m.kind, MemberKind::Place);
            assert_eq!(m.provenance, Provenance::Authored);
            assert!(m.span.is_some());
            let reparsed =
                IrPath::parse(&m.path.to_string(), cadetta_ir::path::PathContext::General).unwrap();
            assert_eq!(reparsed, m.path);
        }
        assert_eq!(ms[0].expr.as_deref(), Some("4mm"));
    }

    // frob:tests crates/cadetta-geometry/src/op_members.rs::instance_members kind=unit
    #[test]
    fn array_placement_lowers_per_instance_with_integer_indexes() {
        let all = lowered();
        let ms = instance_members(&all[1], 4, &from()).unwrap();
        let paths: Vec<String> = ms.iter().map(|m| m.path.to_string()).collect();
        assert_eq!(
            paths,
            [
                "acme.rover::Drive/feet[0].place.offset.x",
                "acme.rover::Drive/feet[1].place.offset.x",
                "acme.rover::Drive/feet[2].place.offset.x",
                "acme.rover::Drive/feet[3].place.offset.x",
            ]
        );
        assert!(ms.iter().all(|m| m.expr.as_deref() == Some("9mm")));
        assert!(ms.iter().all(|m| m.span == ms[0].span));
    }

    // frob:tests crates/cadetta-geometry/src/op_members.rs::ir_path kind=unit
    #[test]
    fn star_is_never_an_ir_index() {
        let mut p = CanonPath::item("acme.rover", &[], "Drive");
        p.parts.push(cadetta_ir::canon::PartSeg {
            name: "feet".into(),
            index: Some(Index::Any),
        });
        p.members.push(MemberSeg::Name("x".into()));
        assert!(ir_path(&p).is_err());
    }
}
