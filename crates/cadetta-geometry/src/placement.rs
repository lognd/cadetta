//! Placement trees: `place A on B`, composition, G002 and G006 (geometry.md 4).
//!
//! `place PART.DATUM on EXPR` makes a datum of one part coincide with a frame of the assembly:
//! `origin`, or a datum of another part or subassembly, optionally built further with
//! `.offset`, `.rotate` and `.flip`. Placements form a tree rooted at the assembly's `origin`, so
//! placing is composition of transforms and never a solve. [`lower_place`] turns the syntax into
//! a [`Placement`] whose operation arguments are [`Member`]s with canonical paths
//! `Assembly/part.place.<op index>.<axis>`; [`check_assembly`] finds the structural errors
//! (G002 cycles, G006 double placement, unplaced parents); [`place_assembly`] evaluates every
//! part's world transform with a caller-supplied member evaluator, so a variant re-evaluates
//! placements by passing the alternate's evaluator, exactly as for any other member.
//!
//! Decisions made here (written into geometry.md 4): a part's world frame is
//! `anchor * ops * inverse(own datum)`, built by appending operations to one frame, with the
//! inverse of a datum being its operations reversed and negated; a subassembly is a node whose
//! datums are its own and whose parts are placed relative to its `origin`, with instance paths
//! joined by `/`; a placement on a part that is itself never placed is G003; the instance of a
//! `for` placement carries the `[*]` index in its canonical path; double placement is judged on
//! the half-open instance ranges of one node, an unindexed target covering all of it.

// frob:ticket 01M4SWFXC2K6D3S93F3ER0GZZA
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use cadetta_sema::resolve::{CanonPath, Index, MemberSeg, PartSeg};
use cadetta_syntax::ast::{Expr, ExprKind, Ident, Path, Place};
use cadetta_syntax::span::Span;
use cadetta_units::{Affine, Dim, Dimension, Magnitude, Rational};
use tracing::{debug, trace, warn};

use crate::datum::{
    Angle, Axis, ComponentId, DatumDef, DatumOp, DatumScope, GeometryError, Member, ORIGIN,
    lower_datum,
};
use crate::interface_datum::resolve_placement_datum;
use crate::transform::Transform;

/// The instances of a node a placement covers, as a half-open range of indices.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Cover {
    /// The whole node (a scalar part, or an array placed as a unit).
    Whole,
    /// Instances `lo..hi` (`feet[0]` is `0..1`; `for i in 0..N` is `0..N`).
    Range {
        /// First instance.
        lo: u64,
        /// One past the last instance.
        hi: u64,
    },
}

impl Cover {
    /// True when the two covers share an instance.
    pub fn overlaps(&self, other: &Cover) -> bool {
        match (self, other) {
            (Cover::Whole, _) | (_, Cover::Whole) => true,
            (Cover::Range { lo: a, hi: b }, Cover::Range { lo: c, hi: d }) => a < d && c < b,
        }
    }
}

/// What a placement attaches to.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Anchor {
    /// The assembly's own `origin`.
    Origin,
    /// A datum of another node of the assembly.
    Datum {
        /// The part or subassembly.
        node: String,
        /// Its datum (`origin` allowed).
        datum: String,
    },
}

/// One lowered `place` statement.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Placement {
    /// The part or subassembly placed.
    pub node: String,
    /// The datum of `node` that is made to coincide.
    pub datum: String,
    /// The instances covered.
    pub cover: Cover,
    /// What it is placed on.
    pub on: Anchor,
    /// The operations built onto the anchor, left to right.
    pub ops: Vec<DatumOp>,
    /// Where it is.
    pub span: Span,
}

impl Placement {
    /// Every operation argument as a member (variants re-evaluate these like any member).
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
}

/// A part or subassembly of an assembly, with the datums it defines.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Node {
    /// The slot name.
    pub name: String,
    /// Its datums (of the slotted implementer, so a swap brings its own).
    pub datums: Vec<DatumDef>,
    /// The assembly inside, when this node is a subassembly.
    pub sub: Option<Box<Assembly>>,
}

/// An assembly: its nodes and its placements.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Assembly {
    /// The assembly (or subassembly) name; the first segment of instance paths.
    pub name: String,
    /// Its parts and subassemblies.
    pub nodes: Vec<Node>,
    /// Its placements, in source order.
    pub placements: Vec<Placement>,
}

/// The rule a placement diagnostic belongs to.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PlacementCode {
    /// G002: a placement chain returns to a part already placed.
    G002,
    /// G003: a placement is on a part that has no placement.
    G003,
    /// G006: a part or instance is placed more than once.
    G006,
    /// A placement names a node or datum the assembly does not have.
    Unresolved,
}

impl PlacementCode {
    /// The catalog code text.
    pub fn code(self) -> &'static str {
        match self {
            PlacementCode::G002 => "G002",
            PlacementCode::G003 => "G003",
            PlacementCode::G006 => "G006",
            PlacementCode::Unresolved => "unresolved",
        }
    }
}

/// A placement diagnostic.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PlacementDiag {
    /// Which rule.
    pub code: PlacementCode,
    /// Where.
    pub span: Span,
    /// What is wrong.
    pub message: String,
}

impl fmt::Display for PlacementDiag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code.code(), self.message)
    }
}

/// A part placed in the world: its instance path, the instances covered and its transform.
#[derive(Clone, PartialEq, Debug)]
pub struct PlacedPart {
    /// The composed instance path, e.g. `Mount/sub/bracket`.
    pub path: String,
    /// The instances placed.
    pub cover: Cover,
    /// The part's origin frame in the root assembly's frame.
    pub transform: Transform,
}

fn err<T>(span: Span, message: impl Into<String>) -> Result<T, GeometryError> {
    let message = message.into();
    warn!(%message, "placement error");
    Err(GeometryError { span, message })
}

fn ident_path(name: &str, span: Span) -> Expr {
    Expr {
        kind: ExprKind::Path(Path {
            segments: vec![Ident {
                name: name.to_owned(),
                span,
            }],
            seps: Vec::new(),
            span,
        }),
        span,
    }
}

/// A reference `node`, `node.datum` or `node[idx].datum` read from an expression.
struct RefParts {
    node: String,
    index: Option<Expr>,
    datum: Option<String>,
}

fn parse_ref(e: &Expr) -> Option<RefParts> {
    match &e.kind {
        ExprKind::Paren(inner) => parse_ref(inner),
        ExprKind::Path(p) if p.segments.len() == 1 => Some(RefParts {
            node: p.segments[0].name.clone(),
            index: None,
            datum: None,
        }),
        ExprKind::Path(p) if p.segments.len() == 2 => Some(RefParts {
            node: p.segments[0].name.clone(),
            index: None,
            datum: Some(p.segments[1].name.clone()),
        }),
        ExprKind::Field(inner, name) => {
            let mut r = parse_ref(inner)?;
            if r.datum.is_some() {
                return None;
            }
            r.datum = Some(name.name.clone());
            Some(r)
        }
        ExprKind::Index(base, idx) => {
            let mut r = parse_ref(base)?;
            if r.index.is_some() || r.datum.is_some() {
                return None;
            }
            r.index = Some((**idx).clone());
            Some(r)
        }
        _ => None,
    }
}

/// Splits `base.op(..).op(..)` into the base reference and the same chain rebased on `origin`.
fn rebase(e: &Expr) -> Option<(Expr, RefParts)> {
    match &e.kind {
        ExprKind::Paren(inner) => rebase(inner),
        ExprKind::Call {
            callee,
            optional: false,
            args,
        } => {
            let (new_callee, base) = match &callee.kind {
                ExprKind::Field(inner, op) => {
                    let (r, base) = rebase(inner)?;
                    (
                        Expr {
                            kind: ExprKind::Field(Box::new(r), op.clone()),
                            span: callee.span,
                        },
                        base,
                    )
                }
                ExprKind::Path(p) if p.segments.len() >= 2 => {
                    let (op, rest) = p.segments.split_last()?;
                    let base = Expr {
                        kind: ExprKind::Path(Path {
                            segments: rest.to_vec(),
                            seps: p.seps[..p.seps.len() - 1].to_vec(),
                            span: p.span,
                        }),
                        span: p.span,
                    };
                    (
                        Expr {
                            kind: ExprKind::Field(Box::new(ident_path(ORIGIN, p.span)), op.clone()),
                            span: callee.span,
                        },
                        parse_ref(&base)?,
                    )
                }
                _ => return None,
            };
            Some((
                Expr {
                    kind: ExprKind::Call {
                        callee: Box::new(new_callee),
                        optional: false,
                        args: args.clone(),
                    },
                    span: e.span,
                },
                base,
            ))
        }
        _ => Some((ident_path(ORIGIN, e.span), parse_ref(e)?)),
    }
}

/// Lowers `place target on expr [for i in lo..hi]`.
///
/// `index` evaluates a literal or `count(..)` bound to an instance number; `None` is an error.
pub fn lower_place(
    id: &ComponentId,
    p: &Place,
    index: &dyn Fn(&Expr) -> Option<u64>,
) -> Result<Placement, GeometryError> {
    let Some(target) = parse_ref(&p.target) else {
        return err(p.target.span, "a placement target is `part.datum`");
    };
    let Some(datum) = target.datum.clone() else {
        return err(
            p.target.span,
            "a placement target names a datum: `part.datum`",
        );
    };
    let cover = match (&p.each, &target.index) {
        (Some(r), _) => match (index(&r.lo), index(&r.hi)) {
            (Some(lo), Some(hi)) => Cover::Range { lo, hi },
            _ => return err(r.var.span, "the bounds of a placement range must be counts"),
        },
        (None, Some(i)) => match index(i) {
            Some(n) => Cover::Range { lo: n, hi: n + 1 },
            None => return err(i.span, "a placed instance index must be a count"),
        },
        (None, None) => Cover::Whole,
    };
    let Some((rebased, base)) = rebase(&p.on) else {
        return err(
            p.on.span,
            "expected a datum with `.offset(..)`, `.rotate(..)` or `.flip(..)`",
        );
    };
    let on = match (base.node.as_str(), base.datum) {
        (ORIGIN, None) => Anchor::Origin,
        (_, Some(d)) => Anchor::Datum {
            node: base.node,
            datum: d,
        },
        (_, None) => {
            return err(
                p.on.span,
                format!("`{}` needs a datum: `{}.datum`", base.node, base.node),
            );
        }
    };
    let mut def = lower_datum(id, "place", &rebased, p.on.span, &DatumScope::new())?;
    let part_seg = PartSeg {
        name: target.node.clone(),
        index: match (&p.each, &target.index) {
            (Some(_), _) => Some(Index::Any),
            (None, Some(i)) => index(i).map(Index::At),
            (None, None) => None,
        },
    };
    for (n, op) in def.ops.iter_mut().enumerate() {
        let fix = |m: &mut Member, axis: Axis| {
            let mut path = CanonPath::item(&id.package, &id.modules, &id.item);
            path.parts.push(part_seg.clone());
            path.members.push(MemberSeg::Name("place".to_owned()));
            path.members.push(MemberSeg::Name(n.to_string()));
            path.members.push(MemberSeg::Name(axis.name().to_owned()));
            m.path = path;
        };
        match op {
            DatumOp::Offset { args } => args.iter_mut().for_each(|(a, m)| fix(m, *a)),
            DatumOp::Rotate {
                axis,
                angle: Angle::Given(m),
            } => fix(m, *axis),
            DatumOp::Rotate { .. } => {}
        }
    }
    debug!(node = %target.node, %datum, ops = def.ops.len(), "lowered placement");
    Ok(Placement {
        node: target.node,
        datum,
        cover,
        on,
        ops: def.ops,
        span: p.span,
    })
}

fn diag(code: PlacementCode, span: Span, message: String) -> PlacementDiag {
    warn!(code = code.code(), %message, "placement diagnostic");
    PlacementDiag {
        code,
        span,
        message,
    }
}

/// Checks the placement tree of `a` and every subassembly: unresolved names, G006, G003, G002.
pub fn check_assembly(a: &Assembly) -> Vec<PlacementDiag> {
    let mut out = Vec::new();
    let names: BTreeSet<&str> = a.nodes.iter().map(|n| n.name.as_str()).collect();
    let mut first: BTreeMap<&str, &Placement> = BTreeMap::new();
    for (i, p) in a.placements.iter().enumerate() {
        if !names.contains(p.node.as_str()) {
            out.push(diag(
                PlacementCode::Unresolved,
                p.span,
                format!("`{}` is not a part of `{}`", p.node, a.name),
            ));
            continue;
        }
        for q in &a.placements[..i] {
            if q.node == p.node && q.cover.overlaps(&p.cover) {
                out.push(diag(
                    PlacementCode::G006,
                    p.span,
                    format!("`{}` is placed more than once", p.node),
                ));
                break;
            }
        }
        first.entry(p.node.as_str()).or_insert(p);
        if let Anchor::Datum { node, .. } = &p.on
            && !names.contains(node.as_str())
        {
            out.push(diag(
                PlacementCode::Unresolved,
                p.span,
                format!("`{node}` is not a part of `{}`", a.name),
            ));
        }
    }
    let mut in_cycle: BTreeSet<&str> = BTreeSet::new();
    for n in &a.nodes {
        let mut seen = vec![n.name.as_str()];
        let mut cur = n.name.as_str();
        while let Some(p) = first.get(cur) {
            let Anchor::Datum { node, .. } = &p.on else {
                break;
            };
            let next = node.as_str();
            if !names.contains(next) {
                break;
            }
            if !first.contains_key(next) {
                out.push(diag(
                    PlacementCode::G003,
                    p.span,
                    format!("`{cur}` is placed on `{next}`, which has no placement"),
                ));
                break;
            }
            if let Some(pos) = seen.iter().position(|s| *s == next) {
                let cycle = &seen[pos..];
                if cycle.iter().all(|c| !in_cycle.contains(c)) {
                    out.push(diag(
                        PlacementCode::G002,
                        p.span,
                        format!(
                            "placement chain returns to `{next}`; write the closing connection as a check (geometry.md 4)"
                        ),
                    ));
                }
                in_cycle.extend(cycle.iter().copied());
                break;
            }
            seen.push(next);
            cur = next;
        }
    }
    for n in &a.nodes {
        if let Some(sub) = &n.sub {
            out.extend(check_assembly(sub));
        }
    }
    out
}

/// One evaluated operation of a frame.
#[derive(Clone, Debug)]
enum FrameOp {
    Offset(Box<[Option<Affine>; 3]>),
    Rotate(Axis, Box<Affine>),
}

fn length_dim() -> Dim {
    Dim::new(Dimension::from_si([1, 0, 0, 0, 0, 0, 0]), 0)
}

fn half_turn() -> Affine {
    Affine::constant(
        Dim::new(Dimension::dimensionless(), 1),
        Magnitude::new(Rational::from_int(1), 1),
    )
}

fn axis_index(a: Axis) -> usize {
    match a {
        Axis::X => 0,
        Axis::Y => 1,
        Axis::Z => 2,
    }
}

type Eval<'a> = &'a dyn Fn(&Member) -> Result<Affine, GeometryError>;

fn eval_ops(ops: &[DatumOp], eval: Eval<'_>) -> Result<Vec<FrameOp>, GeometryError> {
    let mut out = Vec::new();
    for op in ops {
        match op {
            DatumOp::Offset { args } => {
                let mut d: [Option<Affine>; 3] = [None, None, None];
                for (axis, m) in args {
                    d[axis_index(*axis)] = Some(eval(m)?);
                }
                out.push(FrameOp::Offset(Box::new(d)));
            }
            DatumOp::Rotate { axis, angle } => {
                let a = match angle {
                    Angle::HalfTurn => half_turn(),
                    Angle::Given(m) => eval(m)?,
                };
                out.push(FrameOp::Rotate(*axis, Box::new(a)));
            }
        }
    }
    Ok(out)
}

fn invert(ops: &[FrameOp]) -> Vec<FrameOp> {
    ops.iter()
        .rev()
        .map(|op| match op {
            FrameOp::Offset(d) => FrameOp::Offset(Box::new(std::array::from_fn(|i| {
                d[i].as_ref().map(Affine::neg)
            }))),
            FrameOp::Rotate(a, ang) => FrameOp::Rotate(*a, Box::new(ang.neg())),
        })
        .collect()
}

/// The operations from a component's `origin` to datum `name`, following alias bases.
fn datum_ops(
    defs: &[DatumDef],
    name: &str,
    eval: Eval<'_>,
    span: Span,
) -> Result<Vec<FrameOp>, GeometryError> {
    if name == ORIGIN {
        return Ok(Vec::new());
    }
    let Some(def) = resolve_placement_datum(defs, name) else {
        return err(span, format!("no datum `{name}` (geometry.md 4)"));
    };
    let mut out = datum_ops(defs, &def.base, eval, span)?;
    out.extend(eval_ops(&def.ops, eval)?);
    Ok(out)
}

fn fold(ops: &[FrameOp]) -> Result<Transform, GeometryError> {
    let mut t = Transform::identity(length_dim());
    for op in ops {
        let next = match op {
            FrameOp::Offset(d) => t.offset(d.as_ref()),
            FrameOp::Rotate(a, ang) => t.rotate(*a, ang.as_ref()),
        };
        t = match next {
            Ok(t) => t,
            Err(e) => {
                return err(Span::default(), format!("placement transform: {e}"));
            }
        };
    }
    Ok(t)
}

struct Ctx<'a> {
    a: &'a Assembly,
    eval: Eval<'a>,
    base: Vec<FrameOp>,
    memo: BTreeMap<String, Vec<FrameOp>>,
    depth: usize,
}

impl Ctx<'_> {
    fn node(&self, name: &str, span: Span) -> Result<&Node, GeometryError> {
        match self.a.nodes.iter().find(|n| n.name == name) {
            Some(n) => Ok(n),
            None => err(span, format!("`{name}` is not a part of `{}`", self.a.name)),
        }
    }

    /// The frame of `node`'s origin in the assembly's base frame.
    fn node_frame(&mut self, name: &str, span: Span) -> Result<Vec<FrameOp>, GeometryError> {
        if let Some(f) = self.memo.get(name) {
            return Ok(f.clone());
        }
        self.depth += 1;
        if self.depth > self.a.nodes.len() + 1 {
            return err(
                span,
                format!("placement chain through `{name}` is cyclic (G002)"),
            );
        }
        let Some(p) = self.a.placements.iter().find(|p| p.node == name) else {
            return err(span, format!("`{name}` has no placement (G003)"));
        };
        let mut frame = match &p.on {
            Anchor::Origin => self.base.clone(),
            Anchor::Datum { node, datum } => {
                let mut f = self.node_frame(node, p.span)?;
                let n = self.node(node, p.span)?;
                f.extend(datum_ops(&n.datums, datum, self.eval, p.span)?);
                f
            }
        };
        frame.extend(eval_ops(&p.ops, self.eval)?);
        let own = self.node(name, p.span)?;
        let own_ops = datum_ops(&own.datums, &p.datum, self.eval, p.span)?;
        frame.extend(invert(&own_ops));
        self.depth -= 1;
        trace!(node = name, ops = frame.len(), "node frame composed");
        self.memo.insert(name.to_owned(), frame.clone());
        Ok(frame)
    }
}

/// Evaluates the world transform of every placed part of `a`, recursing into subassemblies.
///
/// `eval` turns a member into its value; call it with a variant's evaluator to re-evaluate the
/// placements for that alternate. Run [`check_assembly`] first for the cycle and double
/// placement diagnostics; a cycle found here is also an error.
pub fn place_assembly(a: &Assembly, eval: Eval<'_>) -> Result<Vec<PlacedPart>, GeometryError> {
    let mut out = Vec::new();
    place_into(a, eval, Vec::new(), &a.name, &mut out)?;
    debug!(assembly = %a.name, parts = out.len(), "placed assembly");
    Ok(out)
}

fn place_into(
    a: &Assembly,
    eval: Eval<'_>,
    base: Vec<FrameOp>,
    prefix: &str,
    out: &mut Vec<PlacedPart>,
) -> Result<(), GeometryError> {
    let mut ctx = Ctx {
        a,
        eval,
        base,
        memo: BTreeMap::new(),
        depth: 0,
    };
    for p in &a.placements {
        let frame = ctx.node_frame(&p.node, p.span)?;
        ctx.depth = 0;
        let node = ctx.node(&p.node, p.span)?;
        let path = format!("{prefix}/{}", node.name);
        match &node.sub {
            Some(sub) => place_into(sub, eval, frame, &path, out)?,
            None => out.push(PlacedPart {
                path,
                cover: p.cover,
                transform: fold(&frame)?,
            }),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadetta_syntax::ast::{Component, FileEntry, ItemKind, MemberKind};
    use cadetta_syntax::parser::parse;

    use crate::datum::lower_component_datums;

    fn id(item: &str) -> ComponentId {
        ComponentId {
            package: "acme.rover".into(),
            modules: vec![],
            item: item.into(),
        }
    }

    fn components(src: &str) -> Vec<Component> {
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

    fn lit(e: &Expr) -> Option<u64> {
        match &e.kind {
            ExprKind::Number(n) => n.parse().ok(),
            ExprKind::Paren(i) => lit(i),
            _ => None,
        }
    }

    fn count(e: &Expr) -> Option<u64> {
        lit(e).or(Some(4))
    }

    fn num(e: &Expr) -> Option<Affine> {
        match &e.kind {
            ExprKind::Quantity(s) => {
                let n: i64 = s[0].number.parse().ok()?;
                match s[0].unit.as_str() {
                    "mm" => Some(Affine::constant(
                        length_dim(),
                        Magnitude::new(Rational::new(n, 1000).ok()?, 0),
                    )),
                    "deg" => Some(Affine::constant(
                        Dim::new(Dimension::dimensionless(), 1),
                        Magnitude::new(Rational::new(n, 180).ok()?, 1),
                    )),
                    _ => None,
                }
            }
            ExprKind::Paren(i) | ExprKind::Tolerance(i, _) => num(i),
            ExprKind::Unary(_, i) => num(i).map(|a| a.neg()),
            _ => None,
        }
    }

    fn eval(m: &Member) -> Result<Affine, GeometryError> {
        num(&m.value).ok_or(GeometryError {
            span: m.value.span,
            message: "test eval".into(),
        })
    }

    /// Builds an assembly named by its component from model sources.
    fn build(src: &str, subs: &[(&str, Assembly)]) -> Assembly {
        let comps = components(src);
        let asm = comps
            .iter()
            .rev()
            .find(|c| c.class == cadetta_syntax::ast::ComponentClass::Assembly)
            .unwrap()
            .clone();
        let mut nodes = Vec::new();
        for m in &asm.members {
            if let MemberKind::Part(p) = &m.kind {
                let name = p.name.name.clone();
                let ty = match &p.ty.kind {
                    cadetta_syntax::ast::TypeKind::Path(path) => path.text(),
                    _ => panic!("part type"),
                };
                let sub = subs
                    .iter()
                    .find(|(n, _)| *n == ty)
                    .map(|(_, a)| Box::new(a.clone()));
                let datums = comps
                    .iter()
                    .find(|c| c.name.name == ty)
                    .map(|c| lower_component_datums(&id(&c.name.name), c).0)
                    .unwrap_or_default();
                nodes.push(Node { name, datums, sub });
            }
        }
        let cid = id(&asm.name.name);
        let placements = asm
            .members
            .iter()
            .filter_map(|m| match &m.kind {
                MemberKind::Place(p) => Some(lower_place(&cid, p, &count).unwrap()),
                _ => None,
            })
            .collect();
        Assembly {
            name: asm.name.name.clone(),
            nodes,
            placements,
        }
    }

    const MODELS: &str = "model Rail {\n  datum top = origin.offset(z = 30mm)\n  datum site = top.offset(x = 40mm [+-0.05mm])\n}\nmodel Bracket {\n  datum base = origin.offset(z = 5mm)\n  datum under = origin.flip(x)\n  datum holes = origin.offset(x = 2mm)\n}\n";

    fn src(asm: &str) -> String {
        format!("{MODELS}{asm}")
    }

    fn val(c: &crate::transform::Coord) -> Magnitude {
        match c {
            crate::transform::Coord::Exact(a) => a.center().clone(),
            _ => panic!("inexact"),
        }
    }

    fn mm(n: i64) -> Magnitude {
        Magnitude::new(Rational::new(n, 1000).unwrap(), 0)
    }

    // frob:tests crates/cadetta-geometry/src/placement.rs::place_assembly kind=unit
    #[test]
    fn datum_frames_coincide() {
        let a = build(
            &src(
                "assembly Mount {\n  part rail: Rail\n  part bracket: Bracket\n  place rail.origin on origin\n  place bracket.base on rail.site\n}\n",
            ),
            &[],
        );
        assert!(check_assembly(&a).is_empty());
        let placed = place_assembly(&a, &eval).unwrap();
        let b = &placed[1];
        assert_eq!(b.path, "Mount/bracket");
        // site is x=40, z=30; the bracket's base is 5 above its origin.
        assert_eq!(val(b.transform.trans(0)), mm(40));
        assert_eq!(val(b.transform.trans(2)), mm(25));
    }

    // frob:tests crates/cadetta-geometry/src/placement.rs::place_assembly kind=unit
    #[test]
    fn z_directions_included() {
        let a = build(
            &src(
                "assembly Mount {\n  part rail: Rail\n  part bracket: Bracket\n  place rail.origin on origin\n  place bracket.under on rail.top\n}\n",
            ),
            &[],
        );
        let placed = place_assembly(&a, &eval).unwrap();
        let t = &placed[1].transform;
        // The bracket's under datum is flipped about x, so its z opposes the rail's.
        assert_eq!(val(t.rot(2, 2)), Magnitude::new(Rational::from_int(-1), 0));
        assert_eq!(val(t.rot(1, 1)), Magnitude::new(Rational::from_int(-1), 0));
        assert_eq!(val(t.trans(2)), mm(30));
    }

    // frob:tests crates/cadetta-geometry/src/placement.rs::check_assembly kind=unit
    #[test]
    fn g002_on_cycle() {
        let a = build(
            &src(
                "assembly Mount {\n  part rail: Rail\n  part bracket: Bracket\n  place rail.origin on bracket.base\n  place bracket.base on rail.site\n}\n",
            ),
            &[],
        );
        let d = check_assembly(&a);
        assert_eq!(d.len(), 1, "{d:?}");
        assert_eq!(d[0].code, PlacementCode::G002);
        assert!(place_assembly(&a, &eval).is_err());
    }

    // frob:tests crates/cadetta-geometry/src/placement.rs::check_assembly kind=unit
    #[test]
    fn g002_must_not_fire_on_tree() {
        let a = build(
            &src(
                "assembly Mount {\n  part rail: Rail\n  part bracket: Bracket\n  part other: Bracket\n  place rail.origin on origin\n  place bracket.base on rail.site\n  place other.base on bracket.holes\n}\n",
            ),
            &[],
        );
        assert!(check_assembly(&a).is_empty());
    }

    // frob:tests crates/cadetta-geometry/src/placement.rs::check_assembly kind=unit
    #[test]
    fn g006_on_double_and_overlapping_ranges() {
        let twice = build(
            &src(
                "assembly Mount {\n  part rail: Rail\n  place rail.origin on origin\n  place rail.top on origin\n}\n",
            ),
            &[],
        );
        let d = check_assembly(&twice);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].code, PlacementCode::G006);
        let ranges = build(
            &src(
                "assembly Mount {\n  part rail: Rail\n  part feet[4]: Bracket\n  place rail.origin on origin\n  place feet[i].base on rail.top for i in 0..3\n  place feet[i].base on rail.top for i in 2..4\n}\n",
            ),
            &[],
        );
        let d = check_assembly(&ranges);
        assert_eq!(d.len(), 1, "{d:?}");
        assert_eq!(d[0].code, PlacementCode::G006);
        let disjoint = build(
            &src(
                "assembly Mount {\n  part rail: Rail\n  part feet[4]: Bracket\n  place rail.origin on origin\n  place feet[i].base on rail.top for i in 0..2\n  place feet[i].base on rail.top for i in 2..4\n}\n",
            ),
            &[],
        );
        assert!(check_assembly(&disjoint).is_empty());
    }

    // frob:tests crates/cadetta-geometry/src/placement.rs::place_assembly kind=unit
    #[test]
    fn subassembly_parts_come_along() {
        let sub = build(
            &src("assembly Sub {\n  part bracket: Bracket\n  place bracket.base on origin\n}\n"),
            &[],
        );
        let a = build(
            &src(
                "assembly Mount {\n  part rail: Rail\n  part sub: Sub\n  place rail.origin on origin\n  place sub.origin on rail.site\n}\n",
            ),
            &[("Sub", sub)],
        );
        let placed = place_assembly(&a, &eval).unwrap();
        let paths: Vec<_> = placed.iter().map(|p| p.path.as_str()).collect();
        assert_eq!(paths, ["Mount/rail", "Mount/sub/bracket"]);
        // bracket sits 5 below the sub's origin, which is at the rail's site (z = 30).
        assert_eq!(val(placed[1].transform.trans(0)), mm(40));
        assert_eq!(val(placed[1].transform.trans(2)), mm(25));
    }

    // frob:tests crates/cadetta-geometry/src/placement.rs::lower_place kind=unit
    #[test]
    fn member_paths_are_canonical() {
        let comps = components(&src(
            "assembly Drive {\n  part plate: Bracket\n  place plate.base on origin.offset(x = 4mm).rotate(z = 90deg).offset(y = 1mm)\n}\n",
        ));
        let asm = comps.last().unwrap();
        let MemberKind::Place(p) = &asm
            .members
            .iter()
            .find(|m| matches!(m.kind, MemberKind::Place(_)))
            .unwrap()
            .kind
        else {
            panic!()
        };
        let pl = lower_place(&id("Drive"), p, &count).unwrap();
        let paths: Vec<String> = pl.members().iter().map(|m| m.path.to_string()).collect();
        assert_eq!(
            paths,
            [
                "acme.rover::Drive/plate.place.0.x",
                "acme.rover::Drive/plate.place.1.z",
                "acme.rover::Drive/plate.place.2.y",
            ]
        );
    }

    // frob:tests crates/cadetta-geometry/src/placement.rs::place_assembly kind=unit
    #[test]
    fn variants_re_evaluate_members() {
        let a = build(
            &src(
                "assembly Mount {\n  part rail: Rail\n  part bracket: Bracket\n  place rail.origin on origin\n  place bracket.base on rail.site.offset(x = 10mm)\n}\n",
            ),
            &[],
        );
        let alt = |m: &Member| -> Result<Affine, GeometryError> {
            let a = eval(m)?;
            if m.path.members.contains(&MemberSeg::Name("place".into())) {
                Ok(Affine::constant(length_dim(), mm(20)))
            } else {
                Ok(a)
            }
        };
        let base = place_assembly(&a, &eval).unwrap();
        let swapped = place_assembly(&a, &alt).unwrap();
        assert_eq!(val(base[1].transform.trans(0)), mm(50));
        assert_eq!(val(swapped[1].transform.trans(0)), mm(60));
    }
}
