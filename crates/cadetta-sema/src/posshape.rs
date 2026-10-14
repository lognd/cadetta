//! Static datum check of position expressions: G001 for arithmetic, comparison and `distance`
//! across datums, array + point, and a fixed vector meeting another datum; G003 for re-typing
//! a point of an unplaced part (geometry.md 3 and 7).
//!
//! Values carry no datum at run time, so this pass classifies each operand from its declared
//! type (`Point2 on D`) and replays the operation on the geometry crate's `Pos` algebra with
//! empty coordinate lists: only the datum rules matter here, not the numbers.

use std::collections::{BTreeMap, BTreeSet};

use cadetta_geometry::placement::Assembly;
use cadetta_geometry::point::{
    DatumId, Point, Pos, PositionError, add, point_type, require_same_datum, sub,
};
use cadetta_syntax::ast::{BinOp, Component, Expr, ExprKind, MemberKind, Type};
use tracing::{debug, warn};

use super::geom::{find_part, unwrap_array};
use super::{EvalDiagnostic, strip};
use crate::deps::Severity;
use crate::resolve::{ModuleKey, Resolver};

/// One component's pass: the resolver, the component, the placed part names and the lets so far.
pub(super) struct PosCheck<'a, 'r> {
    resolver: &'a Resolver<'r>,
    comp: &'a Component,
    package: String,
    modules: Vec<String>,
    placed: Option<BTreeSet<String>>,
    env: BTreeMap<String, Pos>,
    /// The findings in source order.
    pub out: Vec<EvalDiagnostic>,
}

impl<'a, 'r> PosCheck<'a, 'r> {
    /// A pass over `comp`; `assembly` supplies the placed part names when it is an assembly.
    pub(super) fn new(
        resolver: &'a Resolver<'r>,
        package: &str,
        modules: &[String],
        comp: &'a Component,
        assembly: Option<&Assembly>,
    ) -> Self {
        Self {
            resolver,
            comp,
            package: package.to_owned(),
            modules: modules.to_vec(),
            placed: assembly.map(|a| a.placements.iter().map(|p| p.node.clone()).collect()),
            env: BTreeMap::new(),
            out: Vec::new(),
        }
    }

    /// Walks every member in order, recording the shape of each `let` and checking each `check`.
    pub(super) fn run(&mut self) {
        let comp = self.comp;
        for m in &comp.members {
            match &m.kind {
                MemberKind::Let(l) => self.bind(&l.name.name, l.ty.as_ref(), &l.value),
                MemberKind::Param(q) => self.bind(&q.name.name, q.ty.as_ref(), &q.value),
                MemberKind::Check(c) => {
                    self.walk(&c.cond);
                }
                _ => {}
            }
        }
        debug!(findings = self.out.len(), "position shape pass finished");
    }

    /// Records the shape of a named value: its declared point type, else the shape of its value.
    fn bind(&mut self, name: &str, ty: Option<&Type>, value: &Expr) {
        let declared = ty.and_then(|t| shape_of_type(t, ""));
        let inferred = self.walk(value);
        if let Some(s) = declared.or(inferred) {
            self.env.insert(name.to_owned(), s);
        }
    }

    fn report(&mut self, code: &'static str, e: &Expr, message: String) {
        warn!(code, %message, "position shape finding");
        self.out.push(EvalDiagnostic {
            module: Some(ModuleKey::new(&self.package, &self.modules)),
            code,
            severity: Severity::Error,
            message,
            span: e.span,
        });
    }

    fn lift(&mut self, e: &Expr, r: Result<Pos, PositionError>) -> Option<Pos> {
        match r {
            Ok(p) => Some(p),
            Err(PositionError::G001 { message }) => {
                self.report("G001", e, format!("G001: {message}"));
                None
            }
            Err(_) => None,
        }
    }

    /// The shape of a reference chain: a name bound by an earlier `let`, or a declared point.
    fn reference(&self, e: &Expr) -> Option<Pos> {
        if let ExprKind::Path(p) = &strip(e).kind
            && let [only] = p.segments.as_slice()
            && let Some(s) = self.env.get(&only.name)
        {
            return Some(s.clone());
        }
        let (ty, prefix, indexed) = self.declared_type(e)?;
        let s = shape_of_type(ty, &prefix)?;
        Some(match (s, indexed) {
            (Pos::Array(mut v), true) => v.pop().unwrap_or(Pos::Array(Vec::new())),
            (s, _) => s,
        })
    }

    /// The declared type of a reference chain, the datum prefix of the parts it crosses, and
    /// whether the last step was indexed.
    fn declared_type(&self, e: &Expr) -> Option<(&'a Type, String, bool)> {
        let chain = super::geom::steps(e)?;
        let (last, slots) = chain.split_last()?;
        let mut comp = self.comp;
        let (mut package, mut modules) = (self.package.clone(), self.modules.clone());
        for (slot, _) in slots {
            let part = find_part(comp, slot)?;
            let (p, c) = self.resolver.part_type(&package, &modules, &part.ty)?;
            (package, modules, comp) = (p.package.clone(), p.modules.clone(), c);
        }
        let ty = comp.members.iter().find_map(|m| match &m.kind {
            MemberKind::Param(q) if q.name.name == last.0 => q.ty.as_ref(),
            MemberKind::Let(l) if l.name.name == last.0 => l.ty.as_ref(),
            _ => None,
        })?;
        let prefix = slots
            .iter()
            .map(|s| s.0.as_str())
            .collect::<Vec<_>>()
            .join("/");
        Some((ty, prefix, last.1.is_some()))
    }

    /// Classifies `e`, reporting every datum violation found inside it.
    fn walk(&mut self, e: &Expr) -> Option<Pos> {
        let e = strip(e);
        match &e.kind {
            ExprKind::Path(_) | ExprKind::Field(..) | ExprKind::Index(..) => self.reference(e),
            ExprKind::Binary(op @ (BinOp::Add | BinOp::Sub), l, r) => {
                let (a, b) = (self.walk(l), self.walk(r));
                let (a, b) = (a?, b?);
                let res = if *op == BinOp::Add {
                    add(&a, &b)
                } else {
                    sub(&a, &b)
                };
                self.lift(e, res)
            }
            ExprKind::Binary(BinOp::In, l, r) => {
                let s = self.walk(l)?;
                let chain = super::geom::steps(r)?;
                let (last, slots) = chain.split_last()?;
                let prefix = slots
                    .iter()
                    .map(|s| s.0.as_str())
                    .collect::<Vec<_>>()
                    .join("/");
                let id = if prefix.is_empty() {
                    last.0.clone()
                } else {
                    format!("{prefix}.{}", last.0)
                };
                Some(retag(&s, &DatumId::new(id)))
            }
            ExprKind::Binary(_, l, r) => {
                let (a, b) = (self.walk(l), self.walk(r));
                a.or(b)
            }
            ExprKind::Compare { first, rest } => {
                let mut prev = self.walk(first);
                for (_, next) in rest {
                    let cur = self.walk(next);
                    if let (Some(Pos::Point(p)), Some(Pos::Point(q))) = (&prev, &cur) {
                        let r = require_same_datum(p, q).map(|()| Pos::Point(p.clone()));
                        self.lift(e, r);
                    }
                    prev = cur;
                }
                None
            }
            ExprKind::Call { callee, args, .. } => {
                let shapes: Vec<Option<Pos>> = args.iter().map(|a| self.walk(&a.value)).collect();
                let named_distance = matches!(&strip(callee).kind,
                    ExprKind::Path(p) if p.segments.len() == 1 && p.segments[0].name == "distance");
                if let (true, [Some(Pos::Point(p)), Some(Pos::Point(q))]) =
                    (named_distance, shapes.as_slice())
                {
                    let r = require_same_datum(p, q).map(|()| Pos::Point(p.clone()));
                    self.lift(e, r);
                }
                None
            }
            ExprKind::As(inner, ty) => {
                let s = self.walk(inner);
                if point_type(unwrap_array(ty)).is_some() {
                    self.check_placed(inner);
                }
                s
            }
            ExprKind::Unary(_, x) | ExprKind::Tolerance(x, _) | ExprKind::IsoClass(x, _) => {
                self.walk(x)
            }
            _ => None,
        }
    }

    /// G003: a point read out of a part that no `place` positions has no assembly frame.
    fn check_placed(&mut self, inner: &Expr) {
        let Some(placed) = &self.placed else { return };
        let Some(chain) = super::geom::steps(inner) else {
            return;
        };
        let Some((slot, _)) = chain.first().filter(|_| chain.len() > 1) else {
            return;
        };
        if find_part(self.comp, slot).is_some() && !placed.contains(slot) {
            let msg = format!(
                "G003: `{slot}` is not placed in the assembly, so its points have no frame to re-type through (geometry.md 3)"
            );
            self.report("G003", inner, msg);
        }
    }
}

/// The shape a declared point type stands for; `prefix` is the part chain its datum is under.
fn shape_of_type(ty: &Type, prefix: &str) -> Option<Pos> {
    let pt = point_type(unwrap_array(ty))?;
    let name = pt.datum.0;
    let id = if prefix.is_empty() {
        name
    } else {
        format!("{prefix}.{name}")
    };
    let point = Pos::Point(Point {
        datum: DatumId::new(id),
        coords: Vec::new(),
    });
    let is_array = !std::ptr::eq(unwrap_array(ty), ty);
    Some(if is_array {
        Pos::Array(vec![point])
    } else {
        point
    })
}

/// The same shape expressed in `datum` (what `in` produces).
fn retag(p: &Pos, datum: &DatumId) -> Pos {
    match p {
        Pos::Point(q) => Pos::Point(Point {
            datum: datum.clone(),
            coords: q.coords.clone(),
        }),
        Pos::Vector(v) => Pos::Vector(v.clone()),
        Pos::Array(items) => Pos::Array(items.iter().map(|i| retag(i, datum)).collect()),
    }
}

#[cfg(test)]
mod tests {
    use crate::eval::tests::{file, run};

    fn codes(src: &str) -> Vec<&'static str> {
        run(&file(src)).diagnostics.iter().map(|d| d.code).collect()
    }

    const TWO: &str = "datum top = origin.offset(z = 5mm)\n  a: Point2 on origin = Point2(1mm, 1mm)\n  b: Point2 on top = Point2(2mm, 2mm)\n";

    // frob:ticket 01M4V436F7JRA8P832RH0EKNH2
    // frob:tests crates/cadetta-sema/src/posshape.rs::PosCheck kind=unit
    #[test]
    fn subtraction_across_datums_is_g001() {
        let c = codes(&format!("model M {{\n  {TWO}  d = a - b\n}}\n"));
        assert!(c.contains(&"G001"), "{c:?}");
    }

    // frob:ticket 01M4V436F7JRA8P832RH0EKNH2
    // frob:tests crates/cadetta-sema/src/posshape.rs::PosCheck kind=unit
    #[test]
    fn distance_across_datums_is_g001() {
        let c = codes(&format!("model M {{\n  {TWO}  d = distance(a, b)\n}}\n"));
        assert!(c.contains(&"G001"), "{c:?}");
    }

    // frob:ticket 01M4V436F7JRA8P832RH0EKNH2
    // frob:tests crates/cadetta-sema/src/posshape.rs::PosCheck kind=unit
    #[test]
    fn comparison_across_datums_is_g001_without_eval_noise() {
        let c = codes(&format!(
            "model M {{\n  {TWO}  check same: a == b \"x\"\n}}\n"
        ));
        assert!(c.contains(&"G001") && !c.contains(&"EVAL"), "{c:?}");
    }

    // frob:ticket 01M4V436F7JRA8P832RH0EKNH2
    // frob:tests crates/cadetta-sema/src/posshape.rs::PosCheck kind=unit
    #[test]
    fn re_expressed_operands_share_a_datum() {
        let c = codes(&format!(
            "model M {{\n  {TWO}  check near: distance(a in top, b in top) <= 0.25mm \"x\"\n}}\n"
        ));
        assert!(!c.contains(&"G001"), "{c:?}");
    }

    // frob:ticket 01M4V436F7JRA8P832RH0EKNH2
    // frob:tests crates/cadetta-sema/src/posshape.rs::PosCheck kind=unit
    #[test]
    fn array_plus_point_is_g001() {
        let src = "model M {\n  holes: [Point2 on origin; 4] = rect_pattern(2, 2, 40mm, 20mm)\n  p: Point2 on origin = Point2(1mm, 1mm)\n  s = holes + p\n}\n";
        assert!(codes(src).contains(&"G001"));
    }
}
