//! Placement op arguments as IR members (geometry.md 4; ir.md 2).
//!
//! [`lower_place`](crate::placement::lower_place) leaves each operation argument as a syntax-level
//! [`Member`] with a sema [`CanonPath`]. This module turns them into IR [`IrMember`]s: path
//! `Assembly/part.place.<op index>.<axis>` (`Assembly/part[i].place...` for a for-placement),
//! kind `Place`, provenance `authored`, so bindings, R001, waivers and `why` attach to them.
//!
//! Decision (written into geometry.md 4): the sema path of a `for` placement carries `[*]`
//! because the instance is not one number; the IR path names the range variable instead
//! (`feet[i]`), because `[*]` is valid only in bindings (ir.md 2).
// frob:ticket 01M4SWFXF34KCT5NCZC6VQ98F7

use cadetta_ir::model::{Member as IrMember, MemberKind, Provenance, SpanRef};
use cadetta_ir::path::{Index as IrIndex, Path as IrPath, Seg, Step};
use cadetta_sema::resolve::{CanonPath, Index, MemberSeg};
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
    /// The `for` variable of a for-placement (`i` in `for i in 0..N`), if any.
    pub range_var: Option<&'a str>,
}

/// Converts a sema canonical path to an IR path, naming `[*]` after the range variable.
pub fn ir_path(p: &CanonPath, range_var: Option<&str>) -> Result<IrPath, String> {
    let index = |i: &Index| -> Result<IrIndex, String> {
        match i {
            Index::At(n) => Ok(IrIndex::Int(*n)),
            Index::Any => range_var
                .map(|v| IrIndex::Var(v.to_owned()))
                .ok_or_else(|| format!("`{p}` has `[*]` but no range variable was given")),
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

fn lower_member(m: &Member, from: &OpSource<'_>) -> Result<IrMember, GeometryError> {
    let span = m.value.span;
    let path = ir_path(&m.path, from.range_var).map_err(|message| {
        warn!(%message, "placement member path");
        GeometryError { span, message }
    })?;
    let mut out = IrMember::new(path, MemberKind::Place);
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

/// Every operation argument of `p` as an authored IR member, in op order.
pub fn op_members(p: &Placement, from: &OpSource<'_>) -> Result<Vec<IrMember>, GeometryError> {
    p.members()
        .into_iter()
        .map(|m| lower_member(m, from))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadetta_syntax::ast::{Expr, ExprKind};
    use cadetta_syntax::ast::{FileEntry, ItemKind, MemberKind as AstMember};
    use cadetta_syntax::parser::parse;

    use crate::datum::ComponentId;
    use crate::placement::lower_place;

    const SRC: &str = "model Bracket {\n  datum base = origin.offset(z = 5mm)\n}\nassembly Drive {\n  part plate: Bracket\n  part feet: [Bracket; 4]\n  place plate.base on origin.offset(x = 4mm).rotate(z = 90deg).offset(y = 1mm)\n  place feet[i].base on origin.offset(x = 9mm) for i in 0..4\n}\n";

    fn count(e: &Expr) -> Option<u64> {
        match &e.kind {
            ExprKind::Number(n) => n.parse().ok(),
            _ => None,
        }
    }

    fn lowered() -> Vec<Vec<IrMember>> {
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
                    let pl = lower_place(&id, p, &count).unwrap();
                    let from = OpSource {
                        file: "drive.cdt",
                        src: SRC,
                        range_var: p.each.as_ref().map(|r| r.var.name.as_str()),
                    };
                    out.push(op_members(&pl, &from).unwrap());
                }
            }
        }
        out
    }

    // frob:tests crates/cadetta-geometry/src/op_members.rs::op_members kind=unit
    #[test]
    fn placement_args_are_authored_place_members() {
        let all = lowered();
        let paths: Vec<String> = all[0].iter().map(|m| m.path.to_string()).collect();
        assert_eq!(
            paths,
            [
                "acme.rover::Drive/plate.place.0.x",
                "acme.rover::Drive/plate.place.1.z",
                "acme.rover::Drive/plate.place.2.y",
            ]
        );
        for m in &all[0] {
            assert_eq!(m.kind, MemberKind::Place);
            assert_eq!(m.provenance, Provenance::Authored);
            assert!(m.span.is_some());
        }
        assert_eq!(all[0][0].expr.as_deref(), Some("4mm"));
    }

    // frob:tests crates/cadetta-geometry/src/op_members.rs::op_members kind=unit
    #[test]
    fn for_placement_path_names_the_range_variable() {
        let all = lowered();
        let m = &all[1][0];
        assert_eq!(m.path.to_string(), "acme.rover::Drive/feet[i].place.0.x");
        let reparsed =
            IrPath::parse(&m.path.to_string(), cadetta_ir::path::PathContext::General).unwrap();
        assert_eq!(reparsed, m.path);
    }

    // frob:tests crates/cadetta-geometry/src/op_members.rs::ir_path kind=unit
    #[test]
    fn star_without_a_variable_is_an_error() {
        let mut p = CanonPath::item("acme.rover", &[], "Drive");
        p.parts.push(cadetta_sema::resolve::PartSeg {
            name: "feet".into(),
            index: Some(Index::Any),
        });
        p.members.push(MemberSeg::Name("x".into()));
        assert!(ir_path(&p, None).is_err());
        assert_eq!(
            ir_path(&p, Some("i")).unwrap().to_string(),
            "acme.rover::Drive/feet[i].x"
        );
    }
}
