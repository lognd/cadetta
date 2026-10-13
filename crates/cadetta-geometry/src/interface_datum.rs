//! Interface datums and datum bounds (geometry.md 2).
//!
//! An interface may declare datums (`datum mount`) and relate them by a bound
//! (`shaft.z == mount.z`). Each implementer defines the datums; a missing one is I001, and a
//! datum bound is checked on the implementer's frames: a violated bound is C001 against that
//! implementer, an undecided one (a non-quarter-turn or toleranced rotation) is C002. A
//! placement on an interface-typed part resolves the datum name in the implementer actually
//! slotted, so a swap brings its own `mount` ([`resolve_placement_datum`]).
//!
//! Decision written into geometry.md 2: datum bounds compare axis directions exactly, and only
//! untoleranced rotations by whole multiples of 90deg give a decided verdict.

// frob:ticket 01M4SWFX6PAJ8AK9NND3070SN1

use cadetta_syntax::ast::{Expr, ExprKind, Segment, UnOp};
use cadetta_variants::interface::Limit;
use cadetta_variants::interface::{
    C001, C002, I001, Implementer, InterfaceDiag, ResolvedInterface, check_implementer,
};
use tracing::{debug, trace, warn};

use crate::datum::{Angle, Axis, DatumDef, DatumOp, ORIGIN};

/// An orientation: column `c` is the image of frame axis `c` in the parent frame, or `None`
/// once an inexact rotation has been applied.
type Frame = Option<[[i32; 3]; 3]>;

const IDENTITY: [[i32; 3]; 3] = [[1, 0, 0], [0, 1, 0], [0, 0, 1]];

fn mul(a: &[[i32; 3]; 3], b: &[[i32; 3]; 3]) -> [[i32; 3]; 3] {
    let mut out = [[0; 3]; 3];
    for (i, row) in out.iter_mut().enumerate() {
        for (j, cell) in row.iter_mut().enumerate() {
            *cell = (0..3).map(|k| a[i][k] * b[k][j]).sum();
        }
    }
    out
}

/// The whole number of quarter turns of an untoleranced literal `N deg`, when it is one.
fn quarter_turns(e: &Expr) -> Option<i64> {
    match &e.kind {
        ExprKind::Paren(inner) => quarter_turns(inner),
        ExprKind::Unary(UnOp::Neg, inner) => quarter_turns(inner).map(|n| -n),
        ExprKind::Quantity(segs) => match segs.as_slice() {
            [Segment { number, unit }] if unit == "deg" => {
                let n: i64 = number.parse().ok()?;
                (n % 90 == 0).then_some(n / 90)
            }
            _ => None,
        },
        _ => None,
    }
}

/// The matrix of `turns` quarter turns about `axis`.
fn rotation(axis: Axis, turns: i64) -> [[i32; 3]; 3] {
    let (c, s) = match turns.rem_euclid(4) {
        0 => (1, 0),
        1 => (0, 1),
        2 => (-1, 0),
        _ => (0, -1),
    };
    match axis {
        Axis::X => [[1, 0, 0], [0, c, -s], [0, s, c]],
        Axis::Y => [[c, 0, s], [0, 1, 0], [-s, 0, c]],
        Axis::Z => [[c, -s, 0], [s, c, 0], [0, 0, 1]],
    }
}

/// The orientation of datum `name` relative to the component's `origin`, following base chains
/// through `defs`; `None` when it is not exact or the datum is unknown.
fn orientation(defs: &[DatumDef], name: &str) -> Frame {
    if name == ORIGIN {
        return Some(IDENTITY);
    }
    let def = defs.iter().find(|d| d.name == name)?;
    let mut frame = orientation(defs, &def.base)?;
    for op in &def.ops {
        let DatumOp::Rotate { axis, angle } = op else {
            continue;
        };
        let turns = match angle {
            Angle::HalfTurn => Some(2),
            Angle::Given(m) => quarter_turns(&m.value),
        };
        let Some(turns) = turns else {
            trace!(datum = name, "inexact rotation; orientation undecided");
            return None;
        };
        frame = mul(&frame, &rotation(*axis, turns));
    }
    Some(frame)
}

/// The column of `frame` for `axis`.
fn column(frame: &[[i32; 3]; 3], axis: Axis) -> [i32; 3] {
    let c = match axis {
        Axis::X => 0,
        Axis::Y => 1,
        Axis::Z => 2,
    };
    [frame[0][c], frame[1][c], frame[2][c]]
}

/// Splits `datum.axis` where `datum` is an interface datum (or `origin`).
fn datum_axis<'a>(iface: &ResolvedInterface, text: &'a str) -> Option<(&'a str, Axis)> {
    let (d, a) = text.split_once('.')?;
    let axis = Axis::parse(a)?;
    (d == ORIGIN || iface.datums.iter().any(|x| x == d)).then_some((d, axis))
}

/// Checks the interface's datums against an implementer: I001 for each missing datum, then C001
/// or C002 for each datum bound (`a.z == b.z`) the implementer's frames do not meet.
pub fn check_interface_datums(
    iface: &ResolvedInterface,
    implementer: &str,
    defs: &[DatumDef],
) -> Vec<InterfaceDiag> {
    let mut out = Vec::new();
    for name in &iface.datums {
        if !defs.iter().any(|d| &d.name == name) {
            warn!(%implementer, datum = %name, "implementer lacks interface datum");
            out.push(InterfaceDiag {
                code: I001,
                implementer: implementer.to_owned(),
                member: name.clone(),
                message: format!(
                    "{implementer} lacks datum `{name}` required by interface {}",
                    iface.name
                ),
                span: cadetta_syntax::ast::Span::default(),
            });
        }
    }
    for b in &iface.bounds {
        let Limit::Eq(lhs, rhs) = &b.limit else {
            continue;
        };
        let (Some((ld, la)), Some((rd, ra))) = (datum_axis(iface, lhs), datum_axis(iface, rhs))
        else {
            continue;
        };
        let known = |d: &str| d == ORIGIN || defs.iter().any(|x| x.name == d);
        if !known(ld) || !known(rd) {
            // The missing datum is already I001.
            continue;
        }
        let code = match (orientation(defs, ld), orientation(defs, rd)) {
            (Some(l), Some(r)) if column(&l, la) == column(&r, ra) => continue,
            (Some(_), Some(_)) => C001,
            _ => C002,
        };
        debug!(%implementer, %lhs, %rhs, code, "datum bound not met");
        let word = if code == C001 {
            "violates"
        } else {
            "is undecided against"
        };
        out.push(InterfaceDiag {
            code,
            implementer: implementer.to_owned(),
            member: lhs.clone(),
            message: format!(
                "{implementer}: `{lhs} == {rhs}` {word} a datum bound of interface {}",
                iface.name
            ),
            span: b.span,
        });
    }
    out
}

/// Implementation checking including datums: [`check_implementer`] plus [`check_interface_datums`].
pub fn check_implementer_datums(
    iface: &ResolvedInterface,
    imp: &Implementer,
    defs: &[DatumDef],
) -> Vec<InterfaceDiag> {
    let mut out = check_implementer(iface, imp);
    out.extend(check_interface_datums(iface, &imp.name, defs));
    out
}

/// Resolves a placement's datum name in the implementer actually slotted, so a swap uses the
/// alternate's own definition; `None` when the implementer lacks it.
pub fn resolve_placement_datum<'a>(defs: &'a [DatumDef], name: &str) -> Option<&'a DatumDef> {
    defs.iter().find(|d| d.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::datum::{ComponentId, lower_component_datums};
    use cadetta_syntax::ast::{FileEntry, ItemKind};
    use cadetta_syntax::parser::parse;
    use cadetta_variants::interface::resolve;

    fn iface(src: &str) -> ResolvedInterface {
        let parsed = parse(src);
        assert!(parsed.diags.is_empty(), "{:?}", parsed.diags);
        for e in &parsed.file.entries {
            if let FileEntry::Item(item) = e
                && let ItemKind::Interface(i) = &item.kind
            {
                return resolve(i, &[], |_| None);
            }
        }
        panic!("no interface");
    }

    fn defs(src: &str) -> Vec<DatumDef> {
        let parsed = parse(src);
        assert!(parsed.diags.is_empty(), "{:?}", parsed.diags);
        for e in &parsed.file.entries {
            if let FileEntry::Item(item) = e
                && let ItemKind::Component(c) = &item.kind
            {
                let id = ComponentId {
                    package: "acme.rover".into(),
                    modules: vec![],
                    item: c.name.name.clone(),
                };
                let (d, errs) = lower_component_datums(&id, c);
                assert!(errs.is_empty(), "{errs:?}");
                return d;
            }
        }
        panic!("no component");
    }

    const FLANGE: &str =
        "interface Flange {\n  datum mount\n  datum shaft\n  shaft.z == mount.z\n}\n";

    // frob:tests crates/cadetta-geometry/src/interface_datum.rs::check_interface_datums kind=unit
    #[test]
    fn missing_datum_fails_implementation_checking() {
        let i = iface(FLANGE);
        let d = defs("ots Motor {\n  datum shaft = origin\n}\n");
        let diags = check_interface_datums(&i, "Motor", &d);
        assert!(diags.iter().any(|x| x.code == I001 && x.member == "mount"));
        assert!(!diags[0].waivable());
    }

    // frob:tests crates/cadetta-geometry/src/interface_datum.rs::check_interface_datums kind=unit
    #[test]
    fn opposite_convention_is_c001_against_that_implementer() {
        let i = iface(FLANGE);
        let bad =
            defs("ots Flipped {\n  datum mount = origin\n  datum shaft = origin.flip(x)\n}\n");
        let diags = check_interface_datums(&i, "Flipped", &bad);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, C001);
        assert_eq!(diags[0].implementer, "Flipped");
        let good = defs(
            "ots Plain {\n  datum mount = origin\n  datum shaft = origin.offset(z = 5mm)\n}\n",
        );
        assert!(check_interface_datums(&i, "Plain", &good).is_empty());
        let turned = defs(
            "ots Turned {\n  datum mount = origin.rotate(x = 180deg)\n  datum shaft = origin.flip(x)\n}\n",
        );
        assert!(check_interface_datums(&i, "Turned", &turned).is_empty());
    }

    // frob:tests crates/cadetta-geometry/src/interface_datum.rs::check_interface_datums kind=unit
    #[test]
    fn inexact_rotation_is_undecided() {
        let i = iface(FLANGE);
        let d = defs(
            "ots Tilted {\n  datum mount = origin\n  datum shaft = origin.rotate(x = 10deg)\n}\n",
        );
        let diags = check_interface_datums(&i, "Tilted", &d);
        assert_eq!(diags[0].code, C002);
    }

    // frob:tests crates/cadetta-geometry/src/interface_datum.rs::resolve_placement_datum kind=unit
    #[test]
    fn swap_uses_the_alternates_own_mount() {
        let a = defs("ots A {\n  datum mount = origin.offset(z = 10mm)\n}\n");
        let b = defs("ots B {\n  datum mount = origin.offset(z = 20mm)\n}\n");
        let ma = resolve_placement_datum(&a, "mount").unwrap();
        let mb = resolve_placement_datum(&b, "mount").unwrap();
        assert_ne!(ma, mb);
        assert_eq!(mb.name, "mount");
        assert!(resolve_placement_datum(&b, "nope").is_none());
    }
}
