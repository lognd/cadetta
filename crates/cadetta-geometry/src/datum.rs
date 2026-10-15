//! Datum declarations and the `Datum` type (geometry.md 2).
//!
//! A datum is a right-handed frame built from another datum of the same component by
//! operations applied left to right. Lowering turns the syntax of `datum NAME = EXPR` into a
//! [`DatumDef`]: a base datum and a list of [`DatumOp`]s whose arguments are [`Member`]s with
//! canonical paths (ir.md 2). No value is evaluated here; the transform arithmetic lives with
//! the datum-transform work.
//!
//! Decisions made here (written into geometry.md 2): a datum may only be built from `origin` or
//! a datum declared earlier in the same component, which makes cycles impossible; the n-th
//! operation of one kind in a chain (n > 0) carries an index segment in its canonical path
//! (`Rail.site.offset[1].x`), the first has none (`Rail.site.offset.x`); `flip(x | y)` takes
//! a bare axis name and has no argument, so it has no member.

// frob:ticket 01M4SWFX374CNSPDDD289Z9BCS

use std::collections::BTreeSet;
use std::fmt;

use cadetta_ir::canon::{CanonPath, Index, MemberSeg};
use cadetta_syntax::ast::{Arg, Component, Expr, ExprKind, MemberKind, Segment};
use cadetta_syntax::span::Span;
use tracing::{debug, trace, warn};

/// The name of the datum every component has (geometry.md 2).
pub const ORIGIN: &str = "origin";

/// The name of the `Datum` parameter type (geometry.md 2).
pub const DATUM_TYPE: &str = "Datum";

/// A frame axis.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Axis {
    /// The x axis.
    X,
    /// The y axis.
    Y,
    /// The z axis (the normal).
    Z,
}

impl Axis {
    /// Parses `x`, `y` or `z`.
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "x" => Some(Axis::X),
            "y" => Some(Axis::Y),
            "z" => Some(Axis::Z),
            _ => None,
        }
    }

    /// The axis letter, as used in canonical paths.
    pub fn name(self) -> &'static str {
        match self {
            Axis::X => "x",
            Axis::Y => "y",
            Axis::Z => "z",
        }
    }
}

/// A geometry error with the span it points at.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct GeometryError {
    /// Where.
    pub span: Span,
    /// What is wrong.
    pub message: String,
}

impl fmt::Display for GeometryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for GeometryError {}

fn err<T>(span: Span, message: impl Into<String>) -> Result<T, GeometryError> {
    let message = message.into();
    warn!(%message, "datum error");
    Err(GeometryError { span, message })
}

/// An offset or rotation argument: a value with a canonical path (geometry.md 2; ir.md 2).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Member {
    /// The canonical path, e.g. `Rail.site.offset.x`.
    pub path: CanonPath,
    /// The argument expression as written (it may carry a tolerance).
    pub value: Expr,
}

/// The angle of a rotation.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Angle {
    /// The implicit 180deg of `flip`; it has no member.
    HalfTurn,
    /// A written angle.
    Given(Box<Member>),
}

/// One datum operation.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum DatumOp {
    /// `.offset(x = .., y = .., z = ..)`, arguments in the order written.
    Offset {
        /// Each given axis and its length.
        args: Vec<(Axis, Member)>,
    },
    /// `.rotate(AXIS = ANGLE)`, or `.flip(AXIS)` which lowers to a half turn.
    Rotate {
        /// The axis rotated about, in the frame built so far.
        axis: Axis,
        /// The angle.
        angle: Angle,
    },
}

/// True for the literal `180deg` with no tolerance.
fn is_literal_half_turn(e: &Expr) -> bool {
    match &e.kind {
        ExprKind::Quantity(segs) => {
            matches!(segs.as_slice(), [Segment { number, unit }] if number == "180" && unit == "deg")
        }
        ExprKind::Paren(inner) => is_literal_half_turn(inner),
        _ => false,
    }
}

impl DatumOp {
    /// True when this is a rotation by exactly the literal 180deg (`flip` or `rotate(.. = 180deg)`).
    pub fn is_half_turn(&self) -> bool {
        match self {
            DatumOp::Rotate {
                angle: Angle::HalfTurn,
                ..
            } => true,
            DatumOp::Rotate {
                angle: Angle::Given(m),
                ..
            } => is_literal_half_turn(&m.value),
            DatumOp::Offset { .. } => false,
        }
    }

    /// True when the operation reverses the frame's normal (a half turn about x or y).
    pub fn reverses_normal(&self) -> bool {
        match self {
            DatumOp::Rotate { axis, .. } => {
                self.is_half_turn() && matches!(axis, Axis::X | Axis::Y)
            }
            DatumOp::Offset { .. } => false,
        }
    }
}

/// A lowered `datum NAME = EXPR`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct DatumDef {
    /// The datum name.
    pub name: String,
    /// The datum of the same component it is built from (`origin` or an earlier datum).
    pub base: String,
    /// Operations, applied left to right, each in the frame built so far.
    pub ops: Vec<DatumOp>,
    /// Where it is declared.
    pub span: Span,
}

impl DatumDef {
    /// Every argument member, in application order.
    pub fn members(&self) -> Vec<&Member> {
        let mut out = Vec::new();
        for op in &self.ops {
            match op {
                DatumOp::Offset { args } => out.extend(args.iter().map(|(_, m)| m)),
                DatumOp::Rotate {
                    angle: Angle::Given(m),
                    ..
                } => out.push(m),
                DatumOp::Rotate { .. } => {}
            }
        }
        out
    }

    /// True when the operations alone reverse the normal relative to the base datum.
    pub fn reverses_normal(&self) -> bool {
        self.ops.iter().filter(|o| o.reverses_normal()).count() % 2 == 1
    }
}

/// Where a component's datums are declared: the canonical path prefix of its members.
#[derive(Clone, Debug)]
pub struct ComponentId {
    /// The dotted package name.
    pub package: String,
    /// The module path below the package root.
    pub modules: Vec<String>,
    /// The component (type) name.
    pub item: String,
}

/// The datums a component has so far; also the scope for `Datum`-typed arguments.
#[derive(Clone, Debug)]
pub struct DatumScope {
    names: BTreeSet<String>,
}

impl Default for DatumScope {
    fn default() -> Self {
        Self::new()
    }
}

impl DatumScope {
    /// A scope holding only `origin`.
    pub fn new() -> Self {
        Self {
            names: BTreeSet::from([ORIGIN.to_owned()]),
        }
    }

    /// Adds a declared datum; false when the name is already taken.
    pub fn declare(&mut self, name: &str) -> bool {
        self.names.insert(name.to_owned())
    }

    /// True when `name` is a datum of the component.
    pub fn contains(&self, name: &str) -> bool {
        self.names.contains(name)
    }

    /// Type-checks an argument for a `Datum` parameter: a datum name of the component.
    pub fn check_arg(&self, arg: &Expr) -> Result<String, GeometryError> {
        let name = match &arg.kind {
            ExprKind::Path(p) if p.segments.len() == 1 => p.segments[0].name.clone(),
            ExprKind::Paren(inner) => return self.check_arg(inner),
            _ => return err(arg.span, "a `Datum` argument must be a datum name"),
        };
        if self.contains(&name) {
            trace!(%name, "Datum argument type-checks");
            Ok(name)
        } else {
            err(
                arg.span,
                format!("`{name}` is not a datum of this component (geometry.md 2)"),
            )
        }
    }
}

/// Lowers the datum declarations of `comp` in order; returns the defs and every error found.
pub fn lower_component_datums(
    id: &ComponentId,
    comp: &Component,
) -> (Vec<DatumDef>, Vec<GeometryError>) {
    let mut scope = DatumScope::new();
    let mut defs = Vec::new();
    let mut errors = Vec::new();
    for member in &comp.members {
        let MemberKind::Datum(d) = &member.kind else {
            continue;
        };
        let name = d.name.name.clone();
        let Some(value) = &d.value else {
            errors.push(GeometryError {
                span: d.span,
                message: format!("datum `{name}` needs a defining expression"),
            });
            continue;
        };
        if scope.contains(&name) {
            errors.push(GeometryError {
                span: d.name.span,
                message: format!("datum `{name}` is already declared"),
            });
            continue;
        }
        match lower_datum(id, &name, value, d.span, &scope) {
            Ok(def) => {
                debug!(datum = %name, base = %def.base, ops = def.ops.len(), "lowered datum");
                scope.declare(&name);
                defs.push(def);
            }
            Err(e) => errors.push(e),
        }
    }
    (defs, errors)
}

/// Lowers one `datum name = expr` against the datums already in `scope`.
pub fn lower_datum(
    id: &ComponentId,
    name: &str,
    value: &Expr,
    span: Span,
    scope: &DatumScope,
) -> Result<DatumDef, GeometryError> {
    // Peel the call chain: ops come out outermost (last applied) first.
    let mut calls: Vec<(&str, &[Arg], Span)> = Vec::new();
    let mut cur = value;
    let base = loop {
        match &cur.kind {
            ExprKind::Paren(inner) => cur = inner,
            ExprKind::Path(p) if p.segments.len() == 1 => break p.segments[0].name.clone(),
            ExprKind::Call {
                callee,
                optional: false,
                args,
            } => match &callee.kind {
                ExprKind::Field(inner, op) => {
                    calls.push((op.name.as_str(), args.as_slice(), cur.span));
                    cur = inner;
                }
                _ => {
                    return err(
                        cur.span,
                        "expected `.offset(..)`, `.rotate(..)` or `.flip(..)`",
                    );
                }
            },
            _ => {
                return err(
                    cur.span,
                    "a datum is built from a datum of the same component by `.offset`, `.rotate` and `.flip`",
                );
            }
        }
    };
    if !scope.contains(&base) {
        return err(
            value.span,
            format!(
                "E002: `{base}` is not `origin` or an earlier datum of this component (geometry.md 2)"
            ),
        );
    }
    calls.reverse();
    let mut ops = Vec::new();
    let mut offsets = 0usize;
    let mut rotations = 0usize;
    for (op, args, cspan) in calls {
        match op {
            "offset" => {
                ops.push(lower_offset(id, name, args, cspan, offsets)?);
                offsets += 1;
            }
            "rotate" => {
                ops.push(lower_rotate(id, name, args, cspan, rotations)?);
                rotations += 1;
            }
            "flip" => {
                ops.push(lower_flip(args, cspan)?);
                rotations += 1;
            }
            other => return err(cspan, format!("unknown datum operation `{other}`")),
        }
    }
    if ops.is_empty() {
        trace!(datum = name, "datum is an alias of its base");
    }
    Ok(DatumDef {
        name: name.to_owned(),
        base,
        ops,
        span,
    })
}

fn member_path(id: &ComponentId, datum: &str, op: &str, n: usize, axis: Axis) -> CanonPath {
    let mut path = CanonPath::item(&id.package, &id.modules, &id.item);
    path.members.push(MemberSeg::Name(datum.to_owned()));
    path.members.push(MemberSeg::Name(op.to_owned()));
    if n > 0 {
        path.members.push(MemberSeg::Index(Index::At(n as u64)));
    }
    path.members.push(MemberSeg::Name(axis.name().to_owned()));
    path
}

fn named_axis(arg: &Arg) -> Result<Axis, GeometryError> {
    arg.name
        .as_ref()
        .and_then(|n| Axis::parse(&n.name))
        .map_or_else(
            || {
                err(
                    arg.span,
                    "expected an axis argument `x = ..`, `y = ..` or `z = ..`",
                )
            },
            Ok,
        )
}

fn lower_offset(
    id: &ComponentId,
    datum: &str,
    args: &[Arg],
    span: Span,
    n: usize,
) -> Result<DatumOp, GeometryError> {
    if args.is_empty() {
        return err(span, "`offset` needs at least one of x, y, z");
    }
    let mut out: Vec<(Axis, Member)> = Vec::new();
    for arg in args {
        let axis = named_axis(arg)?;
        if out.iter().any(|(a, _)| *a == axis) {
            return err(
                arg.span,
                format!("`offset` repeats the axis `{}`", axis.name()),
            );
        }
        let path = member_path(id, datum, "offset", n, axis);
        out.push((
            axis,
            Member {
                path,
                value: arg.value.clone(),
            },
        ));
    }
    Ok(DatumOp::Offset { args: out })
}

fn lower_rotate(
    id: &ComponentId,
    datum: &str,
    args: &[Arg],
    span: Span,
    n: usize,
) -> Result<DatumOp, GeometryError> {
    let [arg] = args else {
        return err(span, "`rotate` takes exactly one of x, y, z");
    };
    let axis = named_axis(arg)?;
    let path = member_path(id, datum, "rotate", n, axis);
    Ok(DatumOp::Rotate {
        axis,
        angle: Angle::Given(Box::new(Member {
            path,
            value: arg.value.clone(),
        })),
    })
}

fn lower_flip(args: &[Arg], span: Span) -> Result<DatumOp, GeometryError> {
    let [arg] = args else {
        return err(span, "`flip` takes one axis, x or y");
    };
    let axis = match (&arg.name, &arg.value.kind) {
        (None, ExprKind::Path(p)) if p.segments.len() == 1 => Axis::parse(&p.segments[0].name),
        _ => None,
    };
    match axis {
        Some(a @ (Axis::X | Axis::Y)) => Ok(DatumOp::Rotate {
            axis: a,
            angle: Angle::HalfTurn,
        }),
        _ => err(
            arg.span,
            "E006: `flip` takes x or y (flipping about z does not reverse the normal)",
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadetta_syntax::ast::{FileEntry, ItemKind};
    use cadetta_syntax::parser::parse;

    fn lower(src: &str) -> (Vec<DatumDef>, Vec<GeometryError>) {
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
                return lower_component_datums(&id, c);
            }
        }
        panic!("no component");
    }

    const RAIL: &str = "model Rail {\n  height = 30mm\n  datum top = origin.offset(z = height)\n  datum site = top.offset(x = 40mm [+-0.05mm])\n}\n";

    // frob:tests crates/cadetta-geometry/src/datum.rs::lower_component_datums kind=unit
    #[test]
    fn datum_builds_on_same_component_datum() {
        let (defs, errs) = lower(RAIL);
        assert!(errs.is_empty(), "{errs:?}");
        assert_eq!(defs[0].base, "origin");
        assert_eq!(defs[1].name, "site");
        assert_eq!(defs[1].base, "top");
        assert!(matches!(&defs[1].ops[..], [DatumOp::Offset { args }] if args[0].0 == Axis::X));
    }

    // frob:tests crates/cadetta-geometry/src/datum.rs::lower_component_datums kind=unit
    #[test]
    fn ops_apply_left_to_right() {
        let (defs, errs) = lower(
            "model M {\n  datum d = origin.offset(x = 1mm).rotate(z = 90deg).offset(y = 2mm)\n}\n",
        );
        assert!(errs.is_empty(), "{errs:?}");
        let kinds: Vec<_> = defs[0]
            .ops
            .iter()
            .map(|o| matches!(o, DatumOp::Offset { .. }))
            .collect();
        assert_eq!(kinds, [true, false, true]);
        let paths: Vec<String> = defs[0]
            .members()
            .iter()
            .map(|m| format!("{:?}", m.path.members))
            .collect();
        assert!(
            paths[2].contains("Index"),
            "second offset is indexed: {paths:?}"
        );
    }

    // frob:tests crates/cadetta-geometry/src/datum.rs::member_path kind=unit
    #[test]
    fn arguments_are_members_with_canonical_paths() {
        let (defs, _) = lower(RAIL);
        let m = defs[1].members();
        assert_eq!(m[0].path.item, "Rail");
        assert_eq!(
            m[0].path.members,
            [
                MemberSeg::Name("site".into()),
                MemberSeg::Name("offset".into()),
                MemberSeg::Name("x".into())
            ]
        );
    }

    // frob:tests crates/cadetta-geometry/src/datum.rs::lower_flip kind=unit
    #[test]
    fn flip_equals_half_turn_and_reverses_normal() {
        let (flip, e1) = lower("model M {\n  datum d = origin.flip(x)\n}\n");
        let (rot, e2) = lower("model M {\n  datum d = origin.rotate(x = 180deg)\n}\n");
        assert!(e1.is_empty() && e2.is_empty(), "{e1:?} {e2:?}");
        let (f, r) = (&flip[0].ops[0], &rot[0].ops[0]);
        assert!(f.is_half_turn() && r.is_half_turn());
        assert!(f.reverses_normal() && r.reverses_normal());
        assert!(flip[0].reverses_normal());
        assert!(flip[0].members().is_empty());
        let (about_z, _) = lower("model M {\n  datum d = origin.rotate(z = 180deg)\n}\n");
        assert!(!about_z[0].reverses_normal());
        let (_, bad) = lower("model M {\n  datum d = origin.flip(z)\n}\n");
        assert_eq!(bad.len(), 1);
        assert!(bad[0].message.starts_with("E006: "));
    }

    // frob:tests crates/cadetta-geometry/src/datum.rs::DatumScope.check_arg kind=unit
    #[test]
    fn datum_parameter_takes_a_datum_name() {
        let (defs, _) = lower(RAIL);
        let mut scope = DatumScope::new();
        for d in &defs {
            scope.declare(&d.name);
        }
        let ok = parse("model M {\n  x = site\n  y = nope\n  z = 3mm\n}\n");
        let ExprOf(values) = ExprOf::of(&ok.file);
        assert_eq!(scope.check_arg(&values[0]).unwrap(), "site");
        assert!(scope.check_arg(&values[1]).is_err());
        assert!(scope.check_arg(&values[2]).is_err());
    }

    struct ExprOf(Vec<Expr>);

    impl ExprOf {
        fn of(file: &cadetta_syntax::ast::File) -> Self {
            let mut out = Vec::new();
            for e in &file.entries {
                if let FileEntry::Item(item) = e
                    && let ItemKind::Component(c) = &item.kind
                {
                    for m in &c.members {
                        if let MemberKind::Param(p) = &m.kind {
                            out.push(p.value.clone());
                        }
                    }
                }
            }
            Self(out)
        }
    }

    // frob:tests crates/cadetta-geometry/src/datum.rs::lower_datum kind=unit
    #[test]
    fn unknown_or_later_base_is_an_error() {
        let (_, errs) = lower("model M {\n  datum a = b.offset(x = 1mm)\n  datum b = origin\n}\n");
        assert_eq!(errs.len(), 1);
        assert!(errs[0].message.starts_with("E002: "));
    }
}
