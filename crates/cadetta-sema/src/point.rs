//! `Point2`, `Point3`, `Vec2`, `Vec3` and the geometry patterns (language.md 3.8).
//!
//! A point or vector is a [`Val::Point`] of two or three Length components
//! (exact quantities, or inexact values when a pattern needed trigonometry).
//! Until datum typing lands (geometry epic, geometry.md 3) points and vectors
//! share one representation, so `p + q`, `p - q` and `p * s` all give a point and
//! `array + point` shifts every element. `rect_pattern`, `circle_pattern` and
//! `distance` are computed here with no trigonometry written by hand.

// frob:ticket 01M4ST7AFZGCN52434BQJRT5MM

use cadetta_syntax::ast::{Arg, BinOp, Expr, ExprKind};
use cadetta_syntax::span::Span;
use cadetta_units::{Affine, Dim, Dimension, Magnitude, Rational};
use tracing::{debug, trace};

use super::func::{sqrt, trig};
use super::{Evaluator, Fail, Quant, R, Scope, Val, diag};

/// Largest pattern count accepted (guards against runaway arrays).
const MAX_COUNT: u64 = 65_536;

/// The dimension of a length.
fn length() -> Dim {
    Dim::new(Dimension::from_si([1, 0, 0, 0, 0, 0, 0]), 0)
}

/// The dimension of an angle.
fn angle() -> Dim {
    Dim::new(Dimension::dimensionless(), 1)
}

/// A dimensionless constant `num / den`.
fn ratio(num: i64, den: i64, span: Span) -> R<Quant> {
    let r = Rational::new(num, den).map_err(|e| diag("EVAL", span, e.to_string()))?;
    Ok(Quant::constant(Dim::default(), Magnitude::new(r, 0)))
}

/// The dimension of a numeric value, if it is one.
fn dim_of(v: &Val) -> Option<&Dim> {
    match v {
        Val::Quantity(q) => Some(q.dim()),
        Val::Inexact(i) => Some(&i.dim),
        _ => None,
    }
}

/// Arithmetic on two numeric values, exact when both are exact.
fn scalar(op: BinOp, a: &Val, b: &Val, span: Span) -> R<Val> {
    if let (Val::Quantity(x), Val::Quantity(y)) = (a, b) {
        let q = match op {
            BinOp::Add => x.add(y, span)?,
            BinOp::Sub => x.sub(y, span)?,
            BinOp::Mul => x.mul(y, span)?,
            _ => x.div(y, span)?,
        };
        return Ok(Val::Quantity(q));
    }
    super::func::inex_binary(op, a, b, span)
}

/// True for a dimensionless number (a scale factor).
fn is_factor(v: &Val) -> bool {
    dim_of(v).is_some_and(|d| *d == Dim::default())
}

/// Combines two values where at least one is a point or an array (language.md 3.8).
pub(super) fn compound(op: BinOp, a: &Val, b: &Val, span: Span) -> R<Val> {
    match (a, b) {
        (Val::Array(_), Val::Array(_)) => Err(diag(
            "EVAL",
            span,
            "arithmetic between two arrays is not defined",
        )),
        (Val::Array(xs), _) => xs
            .iter()
            .map(|x| compound(op, x, b, span))
            .collect::<R<Vec<_>>>()
            .map(Val::Array),
        (_, Val::Array(ys)) => ys
            .iter()
            .map(|y| compound(op, a, y, span))
            .collect::<R<Vec<_>>>()
            .map(Val::Array),
        (Val::Point(p), Val::Point(q)) if matches!(op, BinOp::Add | BinOp::Sub) => {
            if p.len() != q.len() {
                return Err(diag("EVAL", span, "points have different dimensions"));
            }
            p.iter()
                .zip(q)
                .map(|(x, y)| scalar(op, x, y, span))
                .collect::<R<Vec<_>>>()
                .map(Val::Point)
        }
        (Val::Point(p), s) if matches!(op, BinOp::Mul | BinOp::Div) && is_factor(s) => p
            .iter()
            .map(|x| scalar(op, x, s, span))
            .collect::<R<Vec<_>>>()
            .map(Val::Point),
        (s, Val::Point(p)) if op == BinOp::Mul && is_factor(s) => p
            .iter()
            .map(|x| scalar(op, s, x, span))
            .collect::<R<Vec<_>>>()
            .map(Val::Point),
        (Val::Point(_), _) | (_, Val::Point(_)) => Err(diag(
            "EVAL",
            span,
            "a point takes `+ -` with a point and `* /` with a dimensionless number",
        )),
        _ => scalar(op, a, b, span),
    }
}

/// Rotates `(x, y)` by `angle` (exact for special angles).
fn rotate(x: &Val, y: &Val, by: &Quant, span: Span) -> R<(Val, Val)> {
    if by.nominal.is_zero() && by.form.is_exact() {
        return Ok((x.clone(), y.clone()));
    }
    let (c, s) = (trig("cos", by, span)?, trig("sin", by, span)?);
    let xc = scalar(BinOp::Mul, x, &c, span)?;
    let ys = scalar(BinOp::Mul, y, &s, span)?;
    let xs = scalar(BinOp::Mul, x, &s, span)?;
    let yc = scalar(BinOp::Mul, y, &c, span)?;
    Ok((
        scalar(BinOp::Sub, &xc, &ys, span)?,
        scalar(BinOp::Add, &xs, &yc, span)?,
    ))
}

impl<'r, 'p> Evaluator<'r, 'p> {
    /// Evaluates `inner [position d]`: one independent disc variable per `Point2` (geometry.md 3.1).
    ///
    /// E006 when a point is not 2-D or any coordinate already carries a tolerance (a coordinate
    /// tolerance on the same value, a toleranced generator member, or a second `position`).
    // frob:ticket 01M50C0NEZM1R9KYFTRJCJY1Z8
    pub(super) fn position_val(
        &mut self,
        sc: &mut Scope,
        inner: &Expr,
        diameter: &Expr,
        span: Span,
    ) -> R<Val> {
        let v = self.eval(sc, inner)?;
        let d = self.quant(sc, diameter)?;
        d.dim()
            .check_same_dimension(&length())
            .map_err(|e| diag("Q002", diameter.span, e.to_string()))?;
        let radius = d.nominal.mul(&Magnitude::new(
            Rational::new(1, 2).expect("2 is nonzero"),
            0,
        ));
        debug!("evaluator: position tolerance, one disc per point");
        self.position_on(v, &radius, span)
    }

    /// Recurses into arrays and gives every 2-D point of `v` its own disc variable.
    fn position_on(&mut self, v: Val, radius: &Magnitude, span: Span) -> R<Val> {
        match v {
            Val::Array(items) => items
                .into_iter()
                .map(|i| self.position_on(i, radius, span))
                .collect::<R<Vec<_>>>()
                .map(Val::Array),
            Val::Point(cs) if cs.len() == 2 => {
                let var = self.fresh_var();
                let zero = Magnitude::new(Rational::from_int(0), 0);
                let mut out = Vec::with_capacity(2);
                for (k, c) in cs.into_iter().enumerate() {
                    let Val::Quantity(q) = c else {
                        return Err(diag(
                            "E006",
                            span,
                            "a position tolerance needs exact length coordinates",
                        ));
                    };
                    if !q.form.is_exact() {
                        return Err(diag(
                            "E006",
                            span,
                            "a position tolerance cannot combine with another tolerance on the same point",
                        ));
                    }
                    let (a, b) = if k == 0 {
                        (radius.clone(), zero.clone())
                    } else {
                        (zero.clone(), radius.clone())
                    };
                    let disc = Affine::disc(q.dim().clone(), zero.clone(), var, a, b);
                    let form = q
                        .form
                        .add(&disc)
                        .map_err(|e| diag("EVAL", span, e.to_string()))?;
                    out.push(Val::Quantity(Quant {
                        form,
                        nominal: q.nominal,
                    }));
                }
                trace!(?var, "evaluator: point owns a disc variable");
                Ok(Val::Point(out))
            }
            Val::Point(_) => Err(diag(
                "E006",
                span,
                "a position tolerance applies to Point2 only; a Point3 has no axis direction",
            )),
            _ => Err(diag(
                "E006",
                span,
                "a position tolerance applies to a Point2 or an array of Point2",
            )),
        }
    }

    /// Evaluates a whole-number argument (a count or an array length).
    pub(super) fn count_arg(&mut self, sc: &mut Scope, e: &Expr, what: &str) -> R<u64> {
        let q = self.quant(sc, e)?;
        let r = q.nominal.rational();
        let whole =
            q.dim() == &Dim::default() && q.nominal.pi_exp() == 0 && r.denom().to_string() == "1";
        match r.numer().to_string().parse::<u64>() {
            Ok(n) if whole && n <= MAX_COUNT => Ok(n),
            _ => Err(diag(
                "EVAL",
                e.span,
                format!("{what} must be a whole number from 0 to {MAX_COUNT}"),
            )),
        }
    }

    /// Evaluates a pattern count: structural, so an exact whole number of at least 1 written
    /// as a literal or parameter-free constant; anything else is E006 (geometry.md 3).
    // frob:ticket 01M4VB63B9FS1JDEH7V08K12NH
    fn pattern_count(&mut self, sc: &mut Scope, e: &Expr, what: &str) -> R<u64> {
        if !parameter_free(e) {
            return Err(diag(
                "E006",
                e.span,
                format!(
                    "{what} must be known before evaluation (a literal or parameter-free constant)"
                ),
            ));
        }
        match self.count_arg(sc, e, what) {
            Ok(0) => Err(diag("E006", e.span, format!("{what} must be at least 1"))),
            Ok(n) => Ok(n),
            Err(Fail::Diag(d)) => Err(diag("E006", d.span, d.message)),
            Err(other) => Err(other),
        }
    }

    /// A numeric argument that must be a Length.
    fn length_arg(&mut self, sc: &mut Scope, e: &Expr) -> R<Val> {
        let v = self.eval(sc, e)?;
        match dim_of(&v) {
            Some(d) if *d == length() => Ok(v),
            _ => Err(diag("Q002", e.span, "a Length is required here")),
        }
    }

    /// Evaluates `Point2`, `Point3`, `Vec2`, `Vec3`, `rect_pattern`, `circle_pattern`, `distance`.
    pub(super) fn geometry(
        &mut self,
        sc: &mut Scope,
        name: &str,
        args: &[Arg],
        span: Span,
    ) -> R<Val> {
        trace!(name, "point: geometry call");
        match name {
            "Point2" | "Vec2" | "Point3" | "Vec3" => {
                let n = if name.ends_with('2') { 2 } else { 3 };
                let xs = super::func::arity(name, args, n, span)?;
                let comps = xs
                    .iter()
                    .map(|x| self.length_arg(sc, x))
                    .collect::<R<Vec<_>>>()?;
                Ok(Val::Point(comps))
            }
            "distance" => {
                let xs = super::func::arity(name, args, 2, span)?;
                let (p, q) = (self.eval(sc, xs[0])?, self.eval(sc, xs[1])?);
                distance(&p, &q, span)
            }
            "radial" => {
                // frob:ticket 01M4VQXPDA4TEKE455FNYB5TXC
                let xs = super::func::arity(name, args, 1, span)?;
                let p = self.eval(sc, xs[0])?;
                radial(&p, span)
            }
            "circle_pattern" => {
                let xs = super::func::arity(name, args, 3, span)?;
                let n = self.pattern_count(sc, xs[0], "the point count")?;
                let r = self.length_arg(sc, xs[1])?;
                let start = self.quant(sc, xs[2])?;
                circle_pattern(n, &r, &start, span)
            }
            _ => self.rect_pattern(sc, args, span),
        }
    }

    fn rect_pattern(&mut self, sc: &mut Scope, args: &[Arg], span: Span) -> R<Val> {
        let mut pos: Vec<&Expr> = Vec::new();
        let mut rotation = None;
        for a in args {
            match a.name.as_ref().map(|n| n.name.as_str()) {
                None => pos.push(&a.value),
                Some("rotation") => rotation = Some(&a.value),
                Some(other) => {
                    return Err(diag(
                        "E006",
                        a.value.span,
                        format!("`rect_pattern` has no parameter `{other}`"),
                    ));
                }
            }
        }
        if pos.len() != 4 {
            return Err(diag(
                "E006",
                span,
                format!(
                    "`rect_pattern` takes 4 positional arguments, found {}",
                    pos.len()
                ),
            ));
        }
        let nx = self.pattern_count(sc, pos[0], "nx")?;
        let ny = self.pattern_count(sc, pos[1], "ny")?;
        let dx = self.length_arg(sc, pos[2])?;
        let dy = self.length_arg(sc, pos[3])?;
        let rot = match rotation {
            Some(e) => self.quant(sc, e)?,
            None => Quant::constant(angle(), Magnitude::new(Rational::from_int(0), 0)),
        };
        if *rot.dim() != angle() {
            return Err(diag("Q002", span, "`rotation` must be an Angle"));
        }
        debug!(nx, ny, "point: rect_pattern");
        let offset = |i: u64, n: u64| -> R<Val> {
            let (i, n) = (i64::try_from(i), i64::try_from(n));
            let (Ok(i), Ok(n)) = (i, n) else {
                return Err(diag("EVAL", span, "pattern count too large"));
            };
            Ok(Val::Quantity(ratio(2 * i - (n - 1), 2, span)?))
        };
        let mut out = Vec::new();
        for j in 0..ny {
            for i in 0..nx {
                let x = scalar(BinOp::Mul, &dx, &offset(i, nx)?, span)?;
                let y = scalar(BinOp::Mul, &dy, &offset(j, ny)?, span)?;
                let (x, y) = rotate(&x, &y, &rot, span)?;
                out.push(Val::Point(vec![x, y]));
            }
        }
        Ok(Val::Array(out))
    }
}

/// True when `e` mentions no name, call or parameter: only literals and arithmetic.
fn parameter_free(e: &Expr) -> bool {
    match &e.kind {
        ExprKind::Number(_) | ExprKind::Quantity(_) => true,
        ExprKind::Paren(x) | ExprKind::Unary(_, x) => parameter_free(x),
        ExprKind::Binary(_, a, b) => parameter_free(a) && parameter_free(b),
        _ => false,
    }
}

/// `n` points of radius `r` from angle `start`, evenly spaced.
fn circle_pattern(n: u64, r: &Val, start: &Quant, span: Span) -> R<Val> {
    if *start.dim() != angle() {
        return Err(diag("Q002", span, "`start` must be an Angle"));
    }
    debug!(n, "point: circle_pattern");
    let mut out = Vec::new();
    for k in 0..n {
        let (Ok(k), Ok(nn)) = (i64::try_from(k), i64::try_from(n)) else {
            return Err(diag("EVAL", span, "pattern count too large"));
        };
        let step = Rational::new(2 * k, nn).map_err(|e| diag("EVAL", span, e.to_string()))?;
        let step = Quant::constant(angle(), Magnitude::new(step, 1));
        let theta = start.add(&step, span)?;
        let x = scalar(BinOp::Mul, r, &trig("cos", &theta, span)?, span)?;
        let y = scalar(BinOp::Mul, r, &trig("sin", &theta, span)?, span)?;
        out.push(Val::Point(vec![x, y]));
    }
    Ok(Val::Array(out))
}

/// The Euclidean distance between two points of one arity.
/// `radial(p)`: the distance of a `Point3` from the z axis of its datum, `sqrt(x^2 + y^2)`.
// frob:ticket 01M4VQXPDA4TEKE455FNYB5TXC
fn radial(p: &Val, span: Span) -> R<Val> {
    let Val::Point(c) = p else {
        return Err(diag("EVAL", span, "`radial` takes a Point3"));
    };
    if c.len() != 3 {
        return Err(diag("EVAL", span, "`radial` takes a Point3"));
    }
    let mut sum: Option<Val> = None;
    for a in &c[..2] {
        let sq = scalar(BinOp::Mul, a, a, span)?;
        sum = Some(match sum {
            None => sq,
            Some(s) => scalar(BinOp::Add, &s, &sq, span)?,
        });
    }
    sqrt(&sum.expect("two components"), span)
}

fn distance(p: &Val, q: &Val, span: Span) -> R<Val> {
    let (Val::Point(p), Val::Point(q)) = (p, q) else {
        return Err(diag("EVAL", span, "`distance` takes two points"));
    };
    if p.len() != q.len() {
        return Err(diag("EVAL", span, "points have different dimensions"));
    }
    let mut sum: Option<Val> = None;
    for (a, b) in p.iter().zip(q) {
        let d = scalar(BinOp::Sub, a, b, span)?;
        let sq = scalar(BinOp::Mul, &d, &d, span)?;
        sum = Some(match sum {
            None => sq,
            Some(s) => scalar(BinOp::Add, &s, &sq, span)?,
        });
    }
    sqrt(&sum.expect("a point has components"), span)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::Evaluation;
    use crate::eval::tests::{file, run};
    use crate::resolve::{CanonPath, MemberSeg};

    fn val(ev: &Evaluation, member: &str) -> Val {
        let mut p = CanonPath::item("p", &[], "M");
        p.members.push(MemberSeg::Name(member.to_owned()));
        ev.get(&p)
            .unwrap_or_else(|| panic!("{member}: {:?}", ev.diagnostics))
            .value
            .clone()
    }

    /// The nominal of a quantity value as an `n/d` rational.
    fn frac(v: &Val) -> (String, String) {
        let Val::Quantity(q) = v else {
            panic!("not an exact quantity: {v:?}")
        };
        (
            q.nominal.rational().numer().to_string(),
            q.nominal.rational().denom().to_string(),
        )
    }

    fn pt(v: &Val) -> Vec<(String, String)> {
        let Val::Point(c) = v else {
            panic!("not a point: {v:?}")
        };
        c.iter().map(frac).collect()
    }

    fn s(n: &str, d: &str) -> (String, String) {
        (n.to_owned(), d.to_owned())
    }

    fn codes(ev: &Evaluation) -> Vec<&'static str> {
        ev.diagnostics.iter().map(|d| d.code).collect()
    }

    // frob:tests fixed_arrays_hold_n_elements_of_one_kind
    #[test]
    fn fixed_arrays_hold_n_elements_of_one_kind() {
        let f = file(
            "model M {\n  a: [Length; 3] = [1mm, 2mm, 3mm]\n  n = count(a)\n  \
             bad: [Length; 3] = [1mm, 2mm]\n  mixed = [1mm, 2kg]\n}\n",
        );
        let ev = run(&f);
        assert_eq!(frac(&val(&ev, "n")), s("3", "1"));
        assert!(
            ev.get(&{
                let mut p = CanonPath::item("p", &[], "M");
                p.members.push(MemberSeg::Name("bad".to_owned()));
                p
            })
            .is_none()
        );
        assert_eq!(codes(&ev).iter().filter(|c| **c == "EVAL").count(), 2);
    }

    // frob:tests point_arithmetic_gives_points_with_length_members
    #[test]
    fn point_arithmetic_gives_points_with_length_members() {
        let f = file(
            "model M {\n  p = Point2(1mm, 2mm)\n  q = Point2(3mm, 5mm)\n  sum = p + q\n  \
             diff = q - p\n  scaled = p * 3\n  px = q.x\n  py = q.y\n  \
             bad = p * 1mm\n}\n",
        );
        let ev = run(&f);
        assert_eq!(pt(&val(&ev, "sum")), vec![s("1", "250"), s("7", "1000")]);
        assert_eq!(pt(&val(&ev, "diff")), vec![s("1", "500"), s("3", "1000")]);
        assert_eq!(pt(&val(&ev, "scaled")), vec![s("3", "1000"), s("3", "500")]);
        assert_eq!(frac(&val(&ev, "px")), s("3", "1000"));
        assert_eq!(frac(&val(&ev, "py")), s("1", "200"));
        assert!(codes(&ev).contains(&"EVAL"), "{:?}", ev.diagnostics);
    }

    // frob:tests rect_pattern_is_centred_rows_first
    #[test]
    fn rect_pattern_is_centred_rows_first() {
        let f = file("model M {\n  h = rect_pattern(2, 2, 10mm, 20mm)\n}\n");
        let ev = run(&f);
        assert!(ev.diagnostics.is_empty(), "{:?}", ev.diagnostics);
        let Val::Array(a) = val(&ev, "h") else {
            panic!("array")
        };
        let got: Vec<_> = a.iter().map(pt).collect();
        let (l, r, lo, hi) = (s("-1", "200"), s("1", "200"), s("-1", "100"), s("1", "100"));
        assert_eq!(
            got,
            vec![
                vec![l.clone(), lo.clone()],
                vec![r.clone(), lo],
                vec![l, hi.clone()],
                vec![r, hi],
            ]
        );
    }

    // frob:tests circle_pattern_and_distance
    #[test]
    fn circle_pattern_and_distance() {
        let f = file(
            "model M {\n  c = circle_pattern(4, 10mm, 0deg)\n  n = count(c)\n  \
             d = distance(Point2(0mm, 0mm), Point2(3mm, 4mm))\n  \
             e = distance(Point2(0mm, 0mm), Point2(1mm, 1mm))\n  \
             f = circle_pattern(3, 10mm, 0deg)\n}\n",
        );
        let ev = run(&f);
        assert!(ev.diagnostics.is_empty(), "{:?}", ev.diagnostics);
        assert_eq!(frac(&val(&ev, "n")), s("4", "1"));
        let Val::Array(a) = val(&ev, "c") else {
            panic!("array")
        };
        assert_eq!(pt(&a[0]), vec![s("1", "100"), s("0", "1")]);
        assert_eq!(pt(&a[1]), vec![s("0", "1"), s("1", "100")]);
        assert_eq!(frac(&val(&ev, "d")), s("1", "200"));
        let Val::Inexact(i) = val(&ev, "e") else {
            panic!("inexact")
        };
        assert!((i.nominal - 0.001_414_213_562).abs() < 1e-9);
        assert!(matches!(val(&ev, "f"), Val::Array(a) if a.len() == 3));
    }

    // frob:tests array_plus_point_shifts_every_element
    #[test]
    fn array_plus_point_shifts_every_element() {
        let f = file("model M {\n  h = rect_pattern(2, 1, 10mm, 10mm) + Point2(5mm, 0mm)\n}\n");
        let ev = run(&f);
        assert!(ev.diagnostics.is_empty(), "{:?}", ev.diagnostics);
        let Val::Array(a) = val(&ev, "h") else {
            panic!("array")
        };
        assert_eq!(pt(&a[0]), vec![s("0", "1"), s("0", "1")]);
        assert_eq!(pt(&a[1]), vec![s("1", "100"), s("0", "1")]);
    }

    // frob:ticket 01M50C0NEZM1R9KYFTRJCJY1Z8
    // frob:tests crates/cadetta-sema/src/point.rs::position_val kind=unit
    #[test]
    fn position_on_point2_is_allowed_and_e006_cases_fire() {
        let ok = run(&file(
            "model M {\n  p = Point2(1mm, 2mm) [position 0.1mm]\n  \
             a = [Point2(0mm, 0mm), Point2(1mm, 1mm)] [position 0.1mm]\n}\n",
        ));
        assert!(codes(&ok).is_empty(), "{:?}", ok.diagnostics);
        let Val::Point(c) = val(&ok, "p") else {
            panic!("not a point")
        };
        let Val::Quantity(x) = &c[0] else { panic!() };
        let Val::Quantity(y) = &c[1] else { panic!() };
        // One disc per point, shared by both coordinates.
        assert_eq!(x.form.disc_vars().len(), 1);
        assert_eq!(x.form.disc_vars(), y.form.disc_vars());
        let Val::Array(a) = val(&ok, "a") else {
            panic!("not an array")
        };
        let disc = |v: &Val| match v {
            Val::Point(c) => match &c[0] {
                Val::Quantity(q) => q.form.disc_vars()[0],
                _ => panic!(),
            },
            _ => panic!(),
        };
        assert_ne!(disc(&a[0]), disc(&a[1]), "independent discs per element");
        for (src, what) in [
            ("Point3(1mm, 2mm, 3mm) [position 0.1mm]", "point3"),
            (
                "Point2(1mm [+-0.05mm], 2mm) [position 0.1mm]",
                "coordinate tolerance",
            ),
            (
                "Point2(1mm, 2mm) [position 0.1mm] [position 0.1mm]",
                "double position",
            ),
        ] {
            let ev = run(&file(&format!("model M {{\n  p = {src}\n}}\n")));
            assert!(codes(&ev).contains(&"E006"), "{what}: {:?}", ev.diagnostics);
        }
        // A toleranced generator member.
        let ev = run(&file(
            "model M {\n  p = rect_pattern(2, 1, 10mm [+-0.1mm], 10mm) [position 0.1mm]\n}\n",
        ));
        assert!(codes(&ev).contains(&"E006"), "{:?}", ev.diagnostics);
        // A diameter that is not a length is a dimension error.
        let ev = run(&file(
            "model M {\n  p = Point2(1mm, 2mm) [position 0.1s]\n}\n",
        ));
        assert!(codes(&ev).contains(&"Q002"), "{:?}", ev.diagnostics);
    }
}

#[cfg(test)]
mod count_tests {
    use crate::eval::tests::{file, run};

    // frob:tests crates/cadetta-sema/src/point.rs::pattern_count
    #[test]
    fn pattern_counts_are_structural() {
        let ok = run(&file(
            "model M {\n  h = rect_pattern(2 * 2, 1, 10mm, 10mm)\n}\n",
        ));
        assert!(ok.diagnostics.is_empty(), "{:?}", ok.diagnostics);
        for src in [
            "model M {\n  n = 2\n  h = rect_pattern(n, 1, 10mm, 10mm)\n}\n",
            "model M {\n  n = 2\n  h = circle_pattern(n, 10mm, 0deg)\n}\n",
            "model M {\n  h = rect_pattern(0, 1, 10mm, 10mm)\n}\n",
            "model M {\n  h = circle_pattern(2.5, 10mm, 0deg)\n}\n",
        ] {
            let ev = run(&file(src));
            assert_eq!(
                ev.diagnostics.iter().map(|d| d.code).collect::<Vec<_>>(),
                ["E006"],
                "{src}"
            );
        }
    }
}
