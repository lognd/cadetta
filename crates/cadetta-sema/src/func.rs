//! User functions and the standard functions (language.md 3.8, quantities.md 4, 5).
//!
//! A user `fn` is pure and total: its body is a block evaluated in a scope
//! holding only its parameters, and recursion is rejected (call graph, EVAL).
//! The standard functions are positional-only. Inexact results (`sqrt`,
//! trigonometry) are carried as [`Inex`] values: a binary64 nominal with a
//! conservative outward-widened interval and the root operation (Q006); round
//! them (`round_to`) to get an exact quantity back.

// frob:ticket 01M4ST7ASWYQV13AAHHCXVR570
// frob:ticket 01M4ST7AFZGCN52434BQJRT5MM

use std::collections::{BTreeMap, BTreeSet};

use cadetta_syntax::ast::{Arg, BinOp, Expr, ExprKind, FnDecl, Path, UnOp};
use cadetta_syntax::span::Span;
use cadetta_units::inexact::{self, round_value};
use cadetta_units::{
    Dim, Dimension, DisplaySpec, InexactError, InexactValue, Magnitude, Rational, RootOp, Rounding,
    Toleranced, Value, format_value,
};
use tracing::{debug, trace};

use super::tablevals::LOOKUPS;
use super::{Evaluator, Fail, Quant, R, Scope, Val, cmp_mag, diag, round_diag, strip, tol_diag};
use crate::resolve::{CanonPath, Ctx, ItemClass, MemberSeg, ModuleKey, Resolved};

/// Relative outward widening applied to every binary64 interval bound.
const WIDEN: f64 = 4.0 * f64::EPSILON;

/// Deepest chain of definitions followed to find an authored display unit.
const UNIT_DEPTH: usize = 16;

/// An inexact value: binary64 nominal and conservative interval (quantities.md 5.4).
#[derive(Clone, Debug)]
pub struct Inex {
    /// The dimension.
    pub dim: Dim,
    /// The nominal in SI base units.
    pub nominal: f64,
    /// Lower bound of the interval.
    pub lo: f64,
    /// Upper bound of the interval.
    pub hi: f64,
    /// The operation that first made the value inexact (Q006 root).
    pub root: RootOp,
    /// The exact nominal when only the tolerance interval is inexact (a square-aware root).
    pub exact_nominal: Option<Magnitude>,
}

impl PartialEq for Inex {
    fn eq(&self, o: &Self) -> bool {
        self.dim == o.dim
            && self.root == o.root
            && self.exact_nominal == o.exact_nominal
            && self.nominal.to_bits() == o.nominal.to_bits()
            && self.lo.to_bits() == o.lo.to_bits()
            && self.hi.to_bits() == o.hi.to_bits()
    }
}

// Inex never holds a NaN (`inex` rejects non-finite values), so equality is total.
impl Eq for Inex {}

/// Any numeric value as an f64 interval.
struct Opd {
    dim: Dim,
    nom: f64,
    lo: f64,
    hi: f64,
    root: Option<RootOp>,
}

fn opd(v: &Val, span: Span) -> R<Opd> {
    match v {
        Val::Quantity(q) => {
            let (lo, hi) = q.form.bounds().map_err(|e| tol_diag(&e, span))?;
            Ok(Opd {
                dim: q.dim().clone(),
                nom: q.nominal.to_f64(),
                lo: lo.to_f64(),
                hi: hi.to_f64(),
                root: None,
            })
        }
        Val::Inexact(i) => Ok(Opd {
            dim: i.dim.clone(),
            nom: i.nominal,
            lo: i.lo,
            hi: i.hi,
            root: Some(i.root),
        }),
        _ => Err(diag("EVAL", span, "a number is required here")),
    }
}

fn widen(lo: f64, hi: f64) -> (f64, f64) {
    (lo - lo.abs() * WIDEN, hi + hi.abs() * WIDEN)
}

/// Builds an [`Inex`] value, rejecting non-finite results.
fn inex(dim: Dim, nominal: f64, lo: f64, hi: f64, root: RootOp, span: Span) -> R<Val> {
    if !(nominal.is_finite() && lo.is_finite() && hi.is_finite()) {
        return Err(diag("EVAL", span, "the result is not a finite number"));
    }
    let (lo, hi) = if lo <= hi { (lo, hi) } else { (hi, lo) };
    trace!(root = root.name(), nominal, lo, hi, "func: inexact value");
    Ok(Val::Inexact(Inex {
        dim,
        nominal,
        lo,
        hi,
        root,
        exact_nominal: None,
    }))
}

fn min_max(xs: &[f64]) -> (f64, f64) {
    let lo = xs.iter().copied().fold(f64::INFINITY, f64::min);
    let hi = xs.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    (lo, hi)
}

/// Arithmetic where at least one operand is inexact: interval arithmetic in f64.
pub(super) fn inex_binary(op: BinOp, a: &Val, b: &Val, span: Span) -> R<Val> {
    let (x, y) = (opd(a, span)?, opd(b, span)?);
    let root = x.root.or(y.root).unwrap_or(RootOp::MixedPiSum);
    let same = |x: &Opd, y: &Opd| {
        x.dim
            .check_same_dimension(&y.dim)
            .map_err(|e| diag("Q002", span, e.to_string()))
    };
    let (dim, nom, lo, hi) = match op {
        BinOp::Add => {
            same(&x, &y)?;
            (x.dim.clone(), x.nom + y.nom, x.lo + y.lo, x.hi + y.hi)
        }
        BinOp::Sub => {
            same(&x, &y)?;
            (x.dim.clone(), x.nom - y.nom, x.lo - y.hi, x.hi - y.lo)
        }
        BinOp::Mul => {
            let (l, h) = min_max(&[x.lo * y.lo, x.lo * y.hi, x.hi * y.lo, x.hi * y.hi]);
            (x.dim.mul(&y.dim), x.nom * y.nom, l, h)
        }
        BinOp::Div => {
            if y.lo <= 0.0 && y.hi >= 0.0 {
                return Err(diag("EVAL", span, "division by a divisor containing zero"));
            }
            let (l, h) = min_max(&[x.lo / y.lo, x.lo / y.hi, x.hi / y.lo, x.hi / y.hi]);
            (x.dim.div(&y.dim), x.nom / y.nom, l, h)
        }
        _ => {
            return Err(diag(
                "EVAL",
                span,
                "this operator does not take an inexact value",
            ));
        }
    };
    let (lo, hi) = widen(lo, hi);
    inex(dim, nom, lo, hi, root, span)
}

/// Negation of an inexact value.
pub(super) fn inex_neg(i: &Inex) -> Inex {
    Inex {
        dim: i.dim.clone(),
        nominal: -i.nominal,
        lo: -i.hi,
        hi: -i.lo,
        root: i.root,
        exact_nominal: i.exact_nominal.as_ref().map(Magnitude::neg),
    }
}

/// The arguments of a standard function: positional only.
fn positional<'a>(name: &str, args: &'a [Arg], span: Span) -> R<Vec<&'a Expr>> {
    let mut out = Vec::with_capacity(args.len());
    for a in args {
        if let Some(n) = &a.name {
            return Err(diag(
                "E006",
                n.span,
                format!("`{name}` takes positional arguments only"),
            ));
        }
        out.push(&a.value);
    }
    trace!(name, n = out.len(), ?span, "func: positional arguments");
    Ok(out)
}

pub(super) fn arity<'a>(name: &str, args: &'a [Arg], want: usize, span: Span) -> R<Vec<&'a Expr>> {
    let xs = positional(name, args, span)?;
    if xs.len() != want {
        return Err(diag(
            "E006",
            span,
            format!("`{name}` takes {want} argument(s), found {}", xs.len()),
        ));
    }
    Ok(xs)
}

fn one() -> Magnitude {
    Magnitude::new(Rational::one(), 0)
}

fn int(n: i64) -> Magnitude {
    Magnitude::new(Rational::from_int(n), 0)
}

fn is_builtin(name: &str) -> bool {
    matches!(
        name,
        "min"
            | "max"
            | "abs"
            | "sqrt"
            | "cbrt"
            | "sin"
            | "cos"
            | "tan"
            | "atan2"
            | "round_to"
            | "ceil_to"
            | "floor_to"
            | "floor"
            | "ceil"
            | "round"
            | "count"
            | "given"
            | "fmt"
            | "Point2"
            | "Point3"
            | "Vec2"
            | "Vec3"
            | "rect_pattern"
            | "circle_pattern"
            | "distance"
            | "radial"
            | "angle_between"
    )
}

impl<'r, 'p> Evaluator<'r, 'p> {
    /// Evaluates a call: a user `fn` first (it shadows), then a standard function.
    pub(super) fn call(
        &mut self,
        sc: &mut Scope,
        callee: &Expr,
        args: &[Arg],
        span: Span,
    ) -> R<Val> {
        // frob:ticket 01M4TRBJ7D2JTGX6XX94BJ1ZGM
        if let ExprKind::Field(table_expr, method) = &callee.kind
            && LOOKUPS.contains(&method.name.as_str())
        {
            let Val::Table(table) = self.eval(sc, table_expr)? else {
                return Err(diag(
                    "EVAL",
                    callee.span,
                    format!("`.{}` needs a table", method.name),
                ));
            };
            let [arg] = args else {
                return Err(diag(
                    "E006",
                    span,
                    format!("`.{}` takes one argument", method.name),
                ));
            };
            let key = self.eval(sc, &arg.value)?;
            return self.table_lookup(&table, &method.name, &key, span);
        }
        let ExprKind::Path(p) = &callee.kind else {
            return Err(diag(
                "EVAL",
                callee.span,
                "the callee must be a function name",
            ));
        };
        if let Ok(Resolved::Item {
            path,
            class: ItemClass::Fn,
        }) = self.resolver.resolve_path(&sc.ctx, p, None)
        {
            return self.call_user(sc, &path, args, span);
        }
        let name = p.segments[0].name.as_str();
        if p.segments.len() == 1 && is_builtin(name) {
            return self.call_std(sc, name, args, span);
        }
        Err(diag(
            "EVAL",
            callee.span,
            format!("unknown function `{}`", p.text()),
        ))
    }

    fn call_user(&mut self, sc: &mut Scope, path: &CanonPath, args: &[Arg], span: Span) -> R<Val> {
        let Some(f) = self.resolver.function(path) else {
            return Err(diag(
                "EVAL",
                span,
                format!("function `{path}` is not known"),
            ));
        };
        if self.recursive.contains(path) {
            debug!(%path, "func: call to a recursive function, already rejected");
            return Err(Fail::Silent);
        }
        let n = f.params.len();
        let mut bound: Vec<Option<Val>> = vec![None; n];
        let mut next = 0;
        for a in args {
            let idx = match &a.name {
                None => {
                    if next >= n {
                        return Err(diag(
                            "E006",
                            a.span,
                            format!("`{}` takes {n} argument(s)", f.name.name),
                        ));
                    }
                    next += 1;
                    next - 1
                }
                Some(id) => f
                    .params
                    .iter()
                    .position(|p| p.name.name == id.name)
                    .ok_or_else(|| {
                        diag(
                            "E006",
                            id.span,
                            format!("`{}` has no parameter `{}`", f.name.name, id.name),
                        )
                    })?,
            };
            if bound[idx].is_some() {
                return Err(diag(
                    "E006",
                    a.span,
                    format!("parameter `{}` is given twice", f.params[idx].name.name),
                ));
            }
            bound[idx] = Some(self.eval_arg(sc, &f.params[idx], &a.value)?);
        }
        let mut fs = Scope {
            ctx: Ctx {
                package: path.package.clone(),
                module: path.modules.clone(),
                component: None,
                locals: Vec::new(),
            },
            locals: Vec::new(),
        };
        for (p, v) in f.params.iter().zip(bound) {
            let v = match v {
                Some(v) => v,
                None => match &p.default {
                    Some(d) => self
                        .eval(&mut fs, d)
                        .map_err(|e| e.in_module(ModuleKey::of_path(path)))?,
                    None => {
                        return Err(diag(
                            "E006",
                            span,
                            format!("missing argument `{}` of `{}`", p.name.name, f.name.name),
                        ));
                    }
                },
            };
            fs.push(&p.name.name, v);
        }
        debug!(%path, "func: calling user function");
        // The body's spans live in the callee's module, not the caller's.
        self.eval_block(&mut fs, &f.body)
            .map_err(|e| e.in_module(ModuleKey::of_path(path)))
    }

    /// An argument value; a bare variant name is accepted where the parameter is an enum.
    fn eval_arg(&mut self, sc: &mut Scope, p: &cadetta_syntax::ast::ParamDecl, e: &Expr) -> R<Val> {
        if let ExprKind::Path(path) = &strip(e).kind
            && let Ok(Resolved::Variant { enum_path, variant }) =
                self.resolver.resolve_path(&sc.ctx, path, Some(&p.ty))
        {
            return Ok(Val::Variant { enum_path, variant });
        }
        self.eval(sc, e)
    }

    fn call_std(&mut self, sc: &mut Scope, name: &str, args: &[Arg], span: Span) -> R<Val> {
        match name {
            "min" | "max" => self.min_max(sc, name == "max", args, span),
            "abs" => {
                let xs = arity(name, args, 1, span)?;
                let v = self.eval(sc, xs[0])?;
                self.abs(&v, span)
            }
            "sqrt" => {
                let xs = arity(name, args, 1, span)?;
                let v = self.eval(sc, xs[0])?;
                sqrt(&v, span)
            }
            "cbrt" => {
                let xs = arity(name, args, 1, span)?;
                let v = self.eval(sc, xs[0])?;
                cbrt(&v, span)
            }
            "sin" | "cos" | "tan" => {
                let xs = arity(name, args, 1, span)?;
                let q = self.quant(sc, xs[0])?;
                trig(name, &q, span)
            }
            "atan2" => {
                let xs = arity(name, args, 2, span)?;
                let y = self.quant(sc, xs[0])?;
                let x = self.quant(sc, xs[1])?;
                atan2(&y, &x, span)
            }
            "round_to" | "ceil_to" | "floor_to" => {
                let xs = arity(name, args, 2, span)?;
                let x = self.eval(sc, xs[0])?;
                let step = self.quant(sc, xs[1])?;
                let mode = match name {
                    "round_to" => Rounding::Round,
                    "ceil_to" => Rounding::Ceil,
                    _ => Rounding::Floor,
                };
                round_to(&x, &step, mode, span)
            }
            "floor" | "ceil" | "round" => {
                let xs = arity(name, args, 1, span)?;
                let x = self.eval(sc, xs[0])?;
                let mode = match name {
                    "floor" => Rounding::Floor,
                    "ceil" => Rounding::Ceil,
                    _ => Rounding::Round,
                };
                to_count(&x, mode, span)
            }
            // frob:ticket 01M4W3YJCH948JJQ3HY646D1T7
            "given" => self.given(args, span),
            "count" => {
                let xs = arity(name, args, 1, span)?;
                // frob:ticket 01M4W5HZZQ36F4WN08B1EBM7ZH
                if let Some(slot) = self.part_slot_ref(sc, xs[0]) {
                    let n = self.count_slot(sc, &slot, span)?;
                    return Ok(Val::Quantity(Quant::constant(
                        Dim::default(),
                        int(i64::from(n)),
                    )));
                }
                match self.eval(sc, xs[0])? {
                    Val::Array(a) => {
                        let n = i64::try_from(a.len()).unwrap_or(i64::MAX);
                        Ok(Val::Quantity(Quant::constant(Dim::default(), int(n))))
                    }
                    _ => Err(diag("EVAL", xs[0].span, "`count` takes an array")),
                }
            }
            "Point2" | "Point3" | "Vec2" | "Vec3" | "rect_pattern" | "circle_pattern"
            | "distance" | "radial" => self.geometry(sc, name, args, span),
            // frob:ticket 01M4V43BJ1Z4EZTYJXMMCJWA2D
            "angle_between" => self.angle_between(sc, args, span),
            _ => self.fmt(sc, args, span),
        }
    }

    /// `given(name)`: true when the optional argument `name` of the current instance was supplied.
    // frob:ticket 01M4W3YJCH948JJQ3HY646D1T7
    fn given(&mut self, args: &[Arg], span: Span) -> R<Val> {
        let xs = arity("given", args, 1, span)?;
        let ExprKind::Path(p) = &strip(xs[0]).kind else {
            return Err(diag("E006", xs[0].span, "`given` takes an argument name"));
        };
        let (Some(inst), [only]) = (self.cur_inst.as_ref(), p.segments.as_slice()) else {
            return Err(diag(
                "E006",
                span,
                "`given` reads an argument inside a component instance",
            ));
        };
        let mut key = inst.prefix.clone();
        key.members = vec![MemberSeg::Name(only.name.clone())];
        let present = self.defs.contains_key(&key);
        trace!(%key, present, "func: given");
        Ok(Val::Bool(present))
    }

    fn min_max(&mut self, sc: &mut Scope, is_max: bool, args: &[Arg], span: Span) -> R<Val> {
        let name = if is_max { "max" } else { "min" };
        let xs = positional(name, args, span)?;
        if xs.len() < 2 {
            return Err(diag(
                "E006",
                span,
                format!("`{name}` takes at least 2 arguments"),
            ));
        }
        let mut vals = Vec::with_capacity(xs.len());
        for x in xs {
            vals.push(self.eval(sc, x)?);
        }
        if let Some(qs) = vals
            .iter()
            .map(|v| match v {
                Val::Quantity(q) => Some(q),
                _ => None,
            })
            .collect::<Option<Vec<_>>>()
        {
            return self.min_max_quant(&qs, is_max, span);
        }
        let ops = vals.iter().map(|v| opd(v, span)).collect::<R<Vec<Opd>>>()?;
        for o in &ops[1..] {
            ops[0]
                .dim
                .check_same_dimension(&o.dim)
                .map_err(|e| diag("Q002", span, e.to_string()))?;
        }
        let pick =
            |f: fn(&[f64]) -> f64, g: fn(&Opd) -> f64| f(&ops.iter().map(g).collect::<Vec<_>>());
        let (lo, hi, nom) = if is_max {
            (
                pick(|v| min_max(v).1, |o| o.lo),
                pick(|v| min_max(v).1, |o| o.hi),
                pick(|v| min_max(v).1, |o| o.nom),
            )
        } else {
            (
                pick(|v| min_max(v).0, |o| o.lo),
                pick(|v| min_max(v).0, |o| o.hi),
                pick(|v| min_max(v).0, |o| o.nom),
            )
        };
        let root = ops
            .iter()
            .find_map(|o| o.root)
            .unwrap_or(RootOp::MixedPiSum);
        inex(ops[0].dim.clone(), nom, lo, hi, root, span)
    }

    fn min_max_quant(&mut self, qs: &[&Quant], is_max: bool, span: Span) -> R<Val> {
        let dim = qs[0].dim().clone();
        let mut bs = Vec::with_capacity(qs.len());
        for q in qs {
            dim.check_same_dimension(q.dim())
                .map_err(|e| diag("Q002", span, e.to_string()))?;
            bs.push(q.form.bounds().map_err(|e| tol_diag(&e, span))?);
        }
        // A certain winner is returned unchanged, keeping its source variables.
        for (i, (lo, hi)) in bs.iter().enumerate() {
            let wins = bs.iter().enumerate().all(|(j, (l2, h2))| {
                j == i
                    || if is_max {
                        cmp_mag(lo, h2).is_ge()
                    } else {
                        cmp_mag(hi, l2).is_le()
                    }
            });
            if wins {
                trace!(i, is_max, "func: min/max has a certain winner");
                return Ok(Val::Quantity(qs[i].clone()));
            }
        }
        let pick = |a: &Magnitude, b: &Magnitude| {
            if cmp_mag(a, b).is_ge() == is_max {
                a.clone()
            } else {
                b.clone()
            }
        };
        let lo = bs.iter().map(|b| b.0.clone()).reduce(|a, b| pick(&a, &b));
        let hi = bs.iter().map(|b| b.1.clone()).reduce(|a, b| pick(&a, &b));
        let nominal = qs
            .iter()
            .map(|q| q.nominal.clone())
            .reduce(|a, b| pick(&a, &b));
        let (Some(lo), Some(hi), Some(nominal)) = (lo, hi, nominal) else {
            return Err(diag("EVAL", span, "min/max of no values"));
        };
        let t = Toleranced::from_interval(dim, lo, hi).map_err(|e| tol_diag(&e, span))?;
        let var = self.fresh_var();
        let form = t.to_affine(var).map_err(|e| tol_diag(&e, span))?;
        Ok(Val::Quantity(Quant { form, nominal }))
    }

    fn abs(&mut self, v: &Val, span: Span) -> R<Val> {
        match v {
            Val::Quantity(q) => {
                let (lo, hi) = q.form.bounds().map_err(|e| tol_diag(&e, span))?;
                let zero = int(0);
                if cmp_mag(&lo, &zero).is_ge() {
                    return Ok(v.clone());
                }
                if cmp_mag(&hi, &zero).is_le() {
                    return Ok(Val::Quantity(q.neg()));
                }
                let m = if cmp_mag(&lo.neg(), &hi).is_ge() {
                    lo.neg()
                } else {
                    hi
                };
                let nominal = if cmp_mag(&q.nominal, &zero).is_lt() {
                    q.nominal.neg()
                } else {
                    q.nominal.clone()
                };
                let t = Toleranced::from_interval(q.dim().clone(), zero, m)
                    .map_err(|e| tol_diag(&e, span))?;
                let var = self.fresh_var();
                let form = t.to_affine(var).map_err(|e| tol_diag(&e, span))?;
                Ok(Val::Quantity(Quant { form, nominal }))
            }
            Val::Inexact(i) => {
                let (lo, hi) = if i.lo >= 0.0 {
                    (i.lo, i.hi)
                } else if i.hi <= 0.0 {
                    (-i.hi, -i.lo)
                } else {
                    (0.0, (-i.lo).max(i.hi))
                };
                inex(i.dim.clone(), i.nominal.abs(), lo, hi, i.root, span)
            }
            _ => Err(diag("EVAL", span, "`abs` takes a number")),
        }
    }

    /// `fmt(template, args...)`: `{}` placeholders rendered by display rules (quantities.md 6).
    fn fmt(&mut self, sc: &mut Scope, args: &[Arg], span: Span) -> R<Val> {
        let xs = positional("fmt", args, span)?;
        let Some((first, rest)) = xs.split_first() else {
            return Err(diag("E006", span, "`fmt` needs a template"));
        };
        let Val::Text(template) = self.eval(sc, first)? else {
            return Err(diag(
                "EVAL",
                first.span,
                "the `fmt` template must be a string",
            ));
        };
        let pieces = split_template(&template)
            .map_err(|why| diag("E006", first.span, format!("bad `fmt` template: {why}")))?;
        let holes = pieces.iter().filter(|p| p.is_none()).count();
        if holes != rest.len() {
            return Err(diag(
                "E006",
                span,
                format!(
                    "`fmt` template has {holes} placeholder(s) but {} argument(s)",
                    rest.len()
                ),
            ));
        }
        let mut rendered = Vec::with_capacity(rest.len());
        for e in rest {
            let v = self.eval(sc, e)?;
            let unit = self.authored_unit(sc, e, 0);
            rendered.push(self.render(&v, unit.as_deref()));
        }
        let mut it = rendered.into_iter();
        let mut out = String::new();
        for p in pieces {
            match p {
                Some(text) => out.push_str(&text),
                None => out.push_str(&it.next().unwrap_or_default()),
            }
        }
        Ok(Val::Text(out))
    }

    fn render(&self, v: &Val, unit: Option<&str>) -> String {
        match v {
            Val::Text(s) => s.clone(),
            Val::Bool(b) => b.to_string(),
            Val::Variant { variant, .. } => variant.clone(),
            Val::Table(t) => t.item.clone(),
            Val::Component(p) => p.item.clone(),
            Val::Row { table, row } => format!("{}[{row}]", table.item),
            Val::Inexact(i) => format!("{}", i.nominal),
            Val::Array(a) => {
                let items: Vec<String> = a.iter().map(|x| self.render(x, unit)).collect();
                format!("[{}]", items.join(", "))
            }
            Val::Point(c) => {
                let items: Vec<String> = c.iter().map(|x| self.render(x, unit)).collect();
                format!("({})", items.join(", "))
            }
            Val::Quantity(q) => {
                if let Some(u) = unit {
                    match format_value(
                        self.units,
                        &q.nominal,
                        q.dim(),
                        &DisplaySpec::Unit(u.to_owned()),
                    ) {
                        Ok(s) => return s,
                        Err(e) => trace!(unit = u, error = %e, "func: authored unit not usable"),
                    }
                }
                base_render(&q.nominal, q.dim())
            }
        }
    }

    /// The unit the value of `e` was authored in, found syntactically.
    fn authored_unit(&self, sc: &Scope, e: &Expr, depth: usize) -> Option<String> {
        if depth > UNIT_DEPTH {
            return None;
        }
        match &strip(e).kind {
            ExprKind::Quantity(segs) => segs.first().map(|s| s.unit.clone()),
            ExprKind::Unary(UnOp::Neg, x)
            | ExprKind::Tolerance(x, _)
            | ExprKind::Position(x, _) => self.authored_unit(sc, x, depth),
            ExprKind::Binary(BinOp::Add | BinOp::Sub, l, r) => self
                .authored_unit(sc, l, depth)
                .or_else(|| self.authored_unit(sc, r, depth)),
            ExprKind::Path(_) | ExprKind::Field(..) => {
                let key = match self.classify(sc, e).ok()? {
                    super::RefKind::Member { key, .. } => key,
                    _ => return None,
                };
                let def = self.defs.get(&key)?;
                let inner = Scope {
                    ctx: def.ctx.clone(),
                    locals: Vec::new(),
                };
                self.authored_unit(&inner, def.expr, depth + 1)
            }
            _ => None,
        }
    }

    /// Rejects recursive functions (language.md 3.8) by a call-graph search.
    pub(super) fn find_recursive(
        resolver: &crate::resolve::Resolver<'p>,
        fns: &BTreeMap<CanonPath, &'p FnDecl>,
    ) -> Vec<(CanonPath, Span)> {
        let mut edges: BTreeMap<&CanonPath, BTreeSet<CanonPath>> = BTreeMap::new();
        for (path, f) in fns {
            let ctx = Ctx {
                package: path.package.clone(),
                module: path.modules.clone(),
                component: None,
                locals: f.params.iter().map(|p| p.name.name.clone()).collect(),
            };
            let mut calls = Vec::new();
            for p in &f.params {
                if let Some(d) = &p.default {
                    collect_calls(d, &mut calls);
                }
            }
            for l in &f.body.lets {
                collect_calls(&l.value, &mut calls);
            }
            collect_calls(&f.body.tail, &mut calls);
            let set = calls
                .into_iter()
                .filter_map(|c| match resolver.resolve_path(&ctx, c, None) {
                    Ok(Resolved::Item {
                        path,
                        class: ItemClass::Fn,
                    }) => Some(path),
                    _ => None,
                })
                .collect();
            edges.insert(path, set);
        }
        let mut out = Vec::new();
        for (path, f) in fns {
            let mut seen: BTreeSet<&CanonPath> = BTreeSet::new();
            let mut stack: Vec<&CanonPath> = edges
                .get(path)
                .map(|s| s.iter().collect())
                .unwrap_or_default();
            let mut cyclic = false;
            while let Some(n) = stack.pop() {
                if n == path {
                    cyclic = true;
                    break;
                }
                if seen.insert(n)
                    && let Some(next) = edges.get(n)
                {
                    stack.extend(next.iter());
                }
            }
            if cyclic {
                debug!(%path, "func: function is recursive");
                out.push((path.clone(), f.name.span));
            }
        }
        out
    }
}

/// Every callee path in `e`.
fn collect_calls<'e>(e: &'e Expr, out: &mut Vec<&'e Path>) {
    match &e.kind {
        ExprKind::Number(_)
        | ExprKind::Quantity(_)
        | ExprKind::Str(_)
        | ExprKind::Bool(_)
        | ExprKind::Path(_) => {}
        ExprKind::Paren(x)
        | ExprKind::Unary(_, x)
        | ExprKind::As(x, _)
        | ExprKind::IsoClass(x, _)
        | ExprKind::Field(x, _) => collect_calls(x, out),
        ExprKind::Index(x, i) => {
            collect_calls(x, out);
            collect_calls(i, out);
        }
        ExprKind::Array(xs) => xs.iter().for_each(|x| collect_calls(x, out)),
        ExprKind::Binary(_, a, b) | ExprKind::Satisfies(a, b) => {
            collect_calls(a, out);
            collect_calls(b, out);
        }
        ExprKind::Compare { first, rest } => {
            collect_calls(first, out);
            rest.iter().for_each(|(_, x)| collect_calls(x, out));
        }
        ExprKind::Call { callee, args, .. } => {
            if let ExprKind::Path(p) = &callee.kind {
                out.push(p);
            } else {
                collect_calls(callee, out);
            }
            args.iter().for_each(|a| collect_calls(&a.value, out));
        }
        ExprKind::Tolerance(x, terms) => {
            collect_calls(x, out);
            terms.iter().for_each(|t| collect_calls(&t.value, out));
        }
        ExprKind::Position(x, d) => {
            collect_calls(x, out);
            collect_calls(d, out);
        }
        ExprKind::If {
            cond,
            then,
            otherwise,
        } => {
            collect_calls(cond, out);
            collect_block(then, out);
            if let Some(o) = otherwise {
                collect_calls(o, out);
            }
        }
        ExprKind::Match { scrutinee, arms } => {
            collect_calls(scrutinee, out);
            arms.iter().for_each(|a| collect_calls(&a.body, out));
        }
        ExprKind::Block(b) => collect_block(b, out),
    }
}

fn collect_block<'e>(b: &'e cadetta_syntax::ast::Block, out: &mut Vec<&'e Path>) {
    b.lets.iter().for_each(|l| collect_calls(&l.value, out));
    collect_calls(&b.tail, out);
}

/// Splits a `fmt` template: `Some(text)` literal runs and `None` placeholders.
fn split_template(t: &str) -> Result<Vec<Option<String>>, String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut chars = t.chars().peekable();
    while let Some(c) = chars.next() {
        match (c, chars.peek().copied()) {
            ('{', Some('{')) => {
                cur.push('{');
                chars.next();
            }
            ('}', Some('}')) => {
                cur.push('}');
                chars.next();
            }
            ('{', Some('}')) => {
                chars.next();
                out.push(Some(std::mem::take(&mut cur)));
                out.push(None);
            }
            ('{' | '}', _) => return Err(format!("unmatched `{c}`")),
            _ => cur.push(c),
        }
    }
    out.push(Some(cur));
    Ok(out)
}

/// SI-base rendering used when no display unit applies.
fn base_render(m: &Magnitude, dim: &Dim) -> String {
    let n = if m.pi_exp() == 0 {
        m.rational().to_string()
    } else {
        format!("{}", m.to_f64())
    };
    if dim.dimension.is_dimensionless() {
        n
    } else {
        format!("{n} {}", dim.dimension.render())
    }
}

fn root_dim(name: &str, n: i32, d: &Dim, span: Span) -> R<Dim> {
    match d.dimension.root(n) {
        Some(r) if d.angle % n == 0 => Ok(Dim::new(r, d.angle / n)),
        _ => Err(diag(
            "Q011",
            span,
            format!(
                "{name} of dimension `{}` is not a whole dimension",
                d.dimension.render()
            ),
        )),
    }
}

fn sqrt_dim(d: &Dim, span: Span) -> R<Dim> {
    root_dim("sqrt", 2, d, span)
}

fn domain(e: &InexactError, span: Span) -> Fail {
    diag(e.code().unwrap_or("EVAL"), span, e.to_string())
}

pub(super) fn sqrt(v: &Val, span: Span) -> R<Val> {
    match v {
        Val::Quantity(q) => {
            let dim = sqrt_dim(q.dim(), span)?;
            let nv = Value::Exact(q.nominal.clone())
                .sqrt()
                .map_err(|e| domain(&e, span))?;
            if q.form.is_exact()
                && let Value::Exact(m) = &nv
            {
                return Ok(Val::Quantity(Quant::constant(dim, m.clone())));
            }
            let iv = q.form.sqrt().map_err(|e| tol_diag(&e, span))?;
            let root = nv.root().unwrap_or(RootOp::Sqrt);
            let out = inex(dim, nv.to_f64(), iv.lo, iv.hi, root, span)?;
            // A root of an exact nominal keeps that exact nominal: only the interval is inexact.
            Ok(match (out, nv) {
                (Val::Inexact(mut i), Value::Exact(m)) => {
                    i.exact_nominal = Some(m);
                    Val::Inexact(i)
                }
                (other, _) => other,
            })
        }
        Val::Inexact(i) => {
            if i.lo < 0.0 {
                return Err(diag("EVAL", span, "sqrt of a value that may be negative"));
            }
            let dim = sqrt_dim(&i.dim, span)?;
            let (lo, hi) = widen(i.lo.sqrt(), i.hi.sqrt());
            inex(dim, i.nominal.max(0.0).sqrt(), lo, hi, i.root, span)
        }
        _ => Err(diag("EVAL", span, "`sqrt` takes a number")),
    }
}

/// `cbrt(x)`: exact for an exact perfect-cube quantity (quantities.md 4.1), else an
/// inexact value (interval cube root when toleranced) rooted at `cbrt`.
// frob:ticket 01M4YMKHAW1T9QY9FWJNQ0HFFK
pub(super) fn cbrt(v: &Val, span: Span) -> R<Val> {
    match v {
        Val::Quantity(q) => {
            let dim = root_dim("cbrt", 3, q.dim(), span)?;
            let nv = Value::Exact(q.nominal.clone()).cbrt();
            if q.form.is_exact()
                && let Value::Exact(m) = &nv
            {
                return Ok(Val::Quantity(Quant::constant(dim, m.clone())));
            }
            let iv = q.form.cbrt().map_err(|e| tol_diag(&e, span))?;
            let root = nv.root().unwrap_or(RootOp::Cbrt);
            let out = inex(dim, nv.to_f64(), iv.lo, iv.hi, root, span)?;
            // A root of an exact nominal keeps that exact nominal: only the interval is inexact.
            Ok(match (out, nv) {
                (Val::Inexact(mut i), Value::Exact(m)) => {
                    i.exact_nominal = Some(m);
                    Val::Inexact(i)
                }
                (other, _) => other,
            })
        }
        Val::Inexact(i) => {
            let dim = root_dim("cbrt", 3, &i.dim, span)?;
            let (lo, hi) = widen(i.lo.cbrt(), i.hi.cbrt());
            inex(dim, i.nominal.cbrt(), lo, hi, i.root, span)
        }
        _ => Err(diag("EVAL", span, "`cbrt` takes a number")),
    }
}

pub(super) fn trig(name: &str, q: &Quant, span: Span) -> R<Val> {
    if !q.dim().dimension.is_dimensionless() || q.dim().angle != 1 {
        return Err(diag(
            "Q002",
            span,
            format!(
                "`{name}` takes an angle, found dimension `{}`",
                q.dim().dimension.render()
            ),
        ));
    }
    let nv = Value::Exact(q.nominal.clone());
    let (v, root) = match name {
        "sin" => (nv.sin(), RootOp::Sin),
        "cos" => (nv.cos(), RootOp::Cos),
        _ => (nv.tan().map_err(|e| domain(&e, span))?, RootOp::Tan),
    };
    if q.form.is_exact()
        && let Value::Exact(m) = &v
    {
        return Ok(Val::Quantity(Quant::constant(Dim::default(), m.clone())));
    }
    let iv = match name {
        "sin" => q.form.sin(),
        "cos" => q.form.cos(),
        _ => q.form.tan(),
    }
    .map_err(|e| tol_diag(&e, span))?;
    inex(
        Dim::default(),
        v.to_f64(),
        iv.lo,
        iv.hi,
        v.root().unwrap_or(root),
        span,
    )
}

fn atan2(y: &Quant, x: &Quant, span: Span) -> R<Val> {
    y.dim()
        .check_same_dimension(x.dim())
        .map_err(|e| diag("Q002", span, e.to_string()))?;
    let dim = Dim::new(Dimension::dimensionless(), 1);
    let nv = inexact::atan2(
        &Value::Exact(y.nominal.clone()),
        y.dim(),
        &Value::Exact(x.nominal.clone()),
        x.dim(),
    )
    .map_err(|e| domain(&e, span))?;
    if y.form.is_exact()
        && x.form.is_exact()
        && let Value::Exact(m) = &nv
    {
        return Ok(Val::Quantity(Quant::constant(dim, m.clone())));
    }
    let (yl, yh) = y.form.bounds().map_err(|e| tol_diag(&e, span))?;
    let (xl, xh) = x.form.bounds().map_err(|e| tol_diag(&e, span))?;
    let (yl, yh, xl, xh) = (yl.to_f64(), yh.to_f64(), xl.to_f64(), xh.to_f64());
    let origin_inside = yl <= 0.0 && yh >= 0.0 && xl <= 0.0 && xh >= 0.0;
    let crosses_cut = yl < 0.0 && yh > 0.0 && xl < 0.0;
    if origin_inside || crosses_cut {
        return Err(diag(
            "EVAL",
            span,
            "atan2 over this tolerance box contains the origin or crosses the branch cut",
        ));
    }
    let corners = [(yl, xl), (yl, xh), (yh, xl), (yh, xh)].map(|(a, b)| a.atan2(b));
    let (lo, hi) = min_max(&corners);
    let (lo, hi) = widen(lo, hi);
    inex(
        dim,
        nv.to_f64(),
        lo,
        hi,
        nv.root().unwrap_or(RootOp::Atan2),
        span,
    )
}

fn round_to(x: &Val, step: &Quant, mode: Rounding, span: Span) -> R<Val> {
    match x {
        Val::Quantity(q) => {
            let rounded = round_value(
                &Value::Exact(q.nominal.clone()),
                q.dim(),
                &step.nominal,
                step.dim(),
                mode,
            )
            .map_err(|e| round_diag(&e, span))?;
            let delta = super::exact(rounded.sub(&q.nominal), span)?;
            let form = q
                .form
                .add(&cadetta_units::Affine::constant(q.dim().clone(), delta))
                .map_err(|e| tol_diag(&e, span))?;
            trace!(?mode, "func: rounded the nominal, deviations kept");
            Ok(Val::Quantity(Quant {
                form,
                nominal: rounded,
            }))
        }
        Val::Inexact(i) => {
            let rounded = round_value(
                &Value::Inexact(InexactValue {
                    value: i.nominal,
                    root: i.root,
                }),
                &i.dim,
                &step.nominal,
                step.dim(),
                mode,
            )
            .map_err(|e| round_diag(&e, span))?;
            Ok(Val::Quantity(Quant::constant(i.dim.clone(), rounded)))
        }
        _ => Err(diag("EVAL", span, "rounding takes a number")),
    }
}

/// `floor`, `ceil` and `round` of a dimensionless number to an integer Count.
fn to_count(x: &Val, mode: Rounding, span: Span) -> R<Val> {
    let (value, dim) = match x {
        Val::Quantity(q) => (Value::Exact(q.nominal.clone()), q.dim().clone()),
        Val::Inexact(i) => (
            Value::Inexact(InexactValue {
                value: i.nominal,
                root: i.root,
            }),
            i.dim.clone(),
        ),
        _ => {
            return Err(diag(
                "EVAL",
                span,
                "`floor`, `ceil` and `round` take a number",
            ));
        }
    };
    if !dim.dimension.is_dimensionless() {
        return Err(diag(
            "Q002",
            span,
            format!(
                "expected a dimensionless number, found `{}`",
                dim.dimension.render()
            ),
        ));
    }
    let n = round_value(&value, &dim, &one(), &Dim::default(), mode)
        .map_err(|e| round_diag(&e, span))?;
    // A pi-carrying operand is irrational, so its integer is not exactly a Count (Q006).
    if let Val::Quantity(q) = x
        && q.nominal.pi_exp() != 0
        && !q.nominal.is_zero()
    {
        debug!(
            pi_exp = q.nominal.pi_exp(),
            "func: rounding a pi factor is inexact (Q006)"
        );
        let f = n.to_f64();
        return inex(Dim::default(), f, f, f, RootOp::PiRound, span);
    }
    Ok(Val::Quantity(Quant::constant(Dim::default(), n)))
}

#[cfg(test)]
mod tests {
    use super::*;

    // frob:tests template_splits_placeholders_and_escapes
    #[test]
    fn template_splits_placeholders_and_escapes() {
        let p = split_template("{} x {{y}} {}").unwrap();
        assert_eq!(
            p,
            vec![
                Some(String::new()),
                None,
                Some(" x {y} ".to_owned()),
                None,
                Some(String::new())
            ]
        );
        assert!(split_template("a { b").is_err());
    }

    fn cube_dim() -> Dim {
        Dim::new(Dimension::from_si([3, 0, 0, 0, 0, 0, 0]), 0)
    }

    // frob:tests cbrt_is_exact_for_cubes_and_checks_the_dimension
    #[test]
    fn cbrt_is_exact_for_cubes_and_checks_the_dimension() {
        let sp = Span::default();
        let cube = |n: i64, d: i64| {
            let r = Rational::new(n, d).unwrap();
            Val::Quantity(Quant::constant(cube_dim(), Magnitude::new(r, 0)))
        };
        let Ok(Val::Quantity(q)) = cbrt(&cube(8, 27), sp) else {
            panic!("cbrt of a cube is exact");
        };
        assert_eq!(*q.nominal.rational(), Rational::new(2, 3).unwrap());
        assert_eq!(
            *q.dim(),
            Dim::new(Dimension::from_si([1, 0, 0, 0, 0, 0, 0]), 0)
        );
        // Not a perfect cube: an inexact value rooted at cbrt, never an error.
        let Ok(Val::Inexact(i)) = cbrt(&cube(2, 1), sp) else {
            panic!("cbrt of a non-cube is inexact");
        };
        assert_eq!(i.root, RootOp::Cbrt);
        assert!((i.nominal - 2f64.cbrt()).abs() < 1e-12 && i.lo <= i.nominal && i.nominal <= i.hi);
        // Inexact input stays inexact and keeps its root.
        let wide = Val::Inexact(Inex {
            dim: cube_dim(),
            nominal: 8.0,
            lo: 7.0,
            hi: 9.0,
            root: RootOp::Sin,
            exact_nominal: None,
        });
        let Ok(Val::Inexact(j)) = cbrt(&wide, sp) else {
            panic!("cbrt of an inexact is inexact");
        };
        assert_eq!(j.root, RootOp::Sin);
        assert!(j.lo <= 7f64.cbrt() && j.hi >= 9f64.cbrt());
        // Dimension L^2 is not a cube: Q011.
        let sq = Val::Quantity(Quant::constant(
            Dim::new(Dimension::from_si([2, 0, 0, 0, 0, 0, 0]), 0),
            Magnitude::new(Rational::from_int(8), 0),
        ));
        let Err(Fail::Diag(d)) = cbrt(&sq, sp) else {
            panic!("Q011 expected");
        };
        assert_eq!(d.code, "Q011");
    }
}
