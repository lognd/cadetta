//! The Python `Quantity`: exact magnitude, dimension, kind and optional
//! tolerance (docs/spec/bindings.md 3.1, quantities.md).
//!
//! All arithmetic, tolerance propagation and kind rules run in
//! `cadetta-units`; this module only adapts them. Exact values are never
//! stored as binary64: inexact results are a separate, marked body.

// frob:ticket 01M4SV2NGV5P9KBBT7987BZCKQ
// frob:ticket 01M4SV2NMPSM42YZT9MXT4YVMW

use std::sync::LazyLock;
use std::sync::atomic::{AtomicU32, Ordering};

use cadetta_syntax::ast::{BinOp, Expr, ExprKind, MemberKind, TolSign, UnOp};
use cadetta_units::{
    Affine, Dim, Dimension, KindError, KindTable, KindedDim, Magnitude, Rational, RootOp, Sum,
    Toleranced, UnitTable, Value, VarId,
};
use pyo3::create_exception;
use pyo3::exceptions::PyException;
use pyo3::prelude::*;
use pyo3::types::PyNotImplemented;
use tracing::{debug, trace};

create_exception!(
    _core,
    QuantityError,
    PyException,
    "A quantity operation violated the kind, dimension or parse rules."
);

static KINDS: LazyLock<KindTable> = LazyLock::new(KindTable::builtin);
static UNITS: LazyLock<UnitTable> = LazyLock::new(UnitTable::builtin);
static NEXT_VAR: AtomicU32 = AtomicU32::new(0);

/// An error with an optional catalog code (`Q002`, `E001`, ...).
#[derive(Debug, Clone, PartialEq)]
struct QErr {
    code: Option<String>,
    message: String,
}

impl QErr {
    fn new(code: Option<&str>, message: impl Into<String>) -> Self {
        Self {
            code: code.map(str::to_string),
            message: message.into(),
        }
    }

    fn plain(message: impl Into<String>) -> Self {
        Self::new(None, message)
    }
}

impl From<KindError> for QErr {
    fn from(e: KindError) -> Self {
        Self::new(e.code(), e.to_string())
    }
}

impl From<QErr> for PyErr {
    fn from(e: QErr) -> Self {
        debug!(code = ?e.code, message = %e.message, "quantity error raised");
        Python::attach(|py| {
            let err = QuantityError::new_err(e.message.clone());
            // Setting a plain attribute on a fresh exception cannot fail.
            let _ = err.value(py).setattr("code", e.code.clone());
            err
        })
    }
}

fn fresh_var() -> VarId {
    VarId(NEXT_VAR.fetch_add(1, Ordering::Relaxed))
}

fn zero() -> Magnitude {
    Magnitude::new(Rational::from_int(0), 0)
}

fn tol_err(e: impl std::fmt::Display) -> QErr {
    QErr::plain(e.to_string())
}

/// The stored value: exact (an affine form plus its nominal) or marked inexact.
#[derive(Debug, Clone)]
#[expect(
    clippy::large_enum_variant,
    reason = "values are short-lived and rarely held in bulk"
)]
enum Body {
    Exact {
        nominal: Magnitude,
        aff: Affine,
    },
    Inexact {
        nominal: f64,
        lo: f64,
        hi: f64,
        root: RootOp,
    },
}

impl Body {
    fn constant(dim: &Dim, m: Magnitude) -> Self {
        Self::Exact {
            nominal: m.clone(),
            aff: Affine::constant(dim.clone(), m),
        }
    }

    fn point(v: f64, root: RootOp) -> Self {
        Self::Inexact {
            nominal: v,
            lo: v,
            hi: v,
            root,
        }
    }

    /// `(nominal, lo, hi, root)` as binary64 for the inexact fallback path.
    fn interval(&self) -> Result<(f64, f64, f64, Option<RootOp>), QErr> {
        match self {
            Self::Exact { nominal, aff } => {
                let (lo, hi) = aff.bounds().map_err(tol_err)?;
                Ok((nominal.to_f64(), lo.to_f64(), hi.to_f64(), None))
            }
            Self::Inexact {
                nominal,
                lo,
                hi,
                root,
            } => Ok((*nominal, *lo, *hi, Some(*root))),
        }
    }
}

fn hull(vals: &[f64]) -> (f64, f64) {
    let lo = vals.iter().copied().fold(f64::INFINITY, f64::min);
    let hi = vals.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    (lo, hi)
}

/// Binary operations that share the exact/inexact dispatch.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Op {
    Add,
    Sub,
    Mul,
    Div,
}

/// An exact magnitude with dimension, kind and optional tolerance.
#[pyclass(
    module = "cadetta._core",
    name = "Quantity",
    frozen,
    skip_from_py_object
)]
#[derive(Debug, Clone)]
pub struct Quantity {
    kd: KindedDim,
    body: Body,
    role: Option<String>,
}

impl Quantity {
    fn new(kd: KindedDim, body: Body) -> Self {
        Self {
            kd,
            body,
            role: None,
        }
    }

    fn scalar(n: i64) -> Self {
        let kd = KindedDim::anonymous(Dim::default());
        let m = Magnitude::new(Rational::from_int(n), 0);
        let body = Body::constant(&kd.dim, m);
        Self::new(kd, body)
    }

    fn binary(&self, o: &Self, op: Op) -> Result<Self, QErr> {
        let kd = match op {
            Op::Add | Op::Sub => KINDS.combine(&self.kd, &o.kd)?.value,
            Op::Mul => KINDS.mul(&self.kd, &o.kd),
            Op::Div => KINDS.div(&self.kd, &o.kd),
        };
        if let (
            Body::Exact {
                nominal: na,
                aff: aa,
            },
            Body::Exact {
                nominal: nb,
                aff: ab,
            },
        ) = (&self.body, &o.body)
            && let Some(body) = exact_binary(&kd, op, (na, aa), (nb, ab))?
        {
            return Ok(Self::new(kd, body));
        }
        let (na, la, ha, ra) = self.body.interval()?;
        let (nb, lb, hb, rb) = o.body.interval()?;
        let root = ra.or(rb).unwrap_or(RootOp::MixedPiSum);
        trace!(?op, "quantity: inexact interval path");
        let (nominal, (lo, hi)) = match op {
            Op::Add => (na + nb, (la + lb, ha + hb)),
            Op::Sub => (na - nb, (la - hb, ha - lb)),
            Op::Mul => (na * nb, hull(&[la * lb, la * hb, ha * lb, ha * hb])),
            Op::Div => {
                if lb <= 0.0 && hb >= 0.0 {
                    return Err(QErr::plain("divisor interval contains zero"));
                }
                (na / nb, hull(&[la / lb, la / hb, ha / lb, ha / hb]))
            }
        };
        Ok(Self::new(
            kd,
            Body::Inexact {
                nominal,
                lo,
                hi,
                root,
            },
        ))
    }

    fn negated(&self) -> Self {
        let body = match &self.body {
            Body::Exact { nominal, aff } => Body::Exact {
                nominal: nominal.neg(),
                aff: aff.neg(),
            },
            Body::Inexact {
                nominal,
                lo,
                hi,
                root,
            } => Body::Inexact {
                nominal: -nominal,
                lo: -hi,
                hi: -lo,
                root: *root,
            },
        };
        Self {
            kd: self.kd.clone(),
            body,
            role: self.role.clone(),
        }
    }

    fn power(&self, n: i32) -> Result<Self, QErr> {
        let kd = KINDS.powi(&self.kd, n);
        if n == 0 {
            return Ok(Self::new(
                kd.clone(),
                Body::constant(&kd.dim, Magnitude::new(Rational::one(), 0)),
            ));
        }
        let mut acc = self.clone();
        for _ in 1..n.unsigned_abs() {
            acc = acc.binary(self, Op::Mul)?;
        }
        if n < 0 {
            acc = Self::scalar(1).binary(&acc, Op::Div)?;
        }
        Ok(acc)
    }

    fn square_root(&self) -> Result<Self, QErr> {
        let kd = KINDS.root(&self.kd, 2)?;
        match &self.body {
            Body::Exact { nominal, aff } if aff.is_exact() => {
                let body = match Value::Exact(nominal.clone()).sqrt().map_err(tol_err)? {
                    Value::Exact(m) => Body::constant(&kd.dim, m),
                    Value::Inexact(i) => Body::point(i.value, i.root),
                };
                Ok(Self::new(kd, body))
            }
            Body::Exact { nominal, aff } => {
                let iv = aff.sqrt().map_err(tol_err)?;
                let nom = Value::Exact(nominal.clone())
                    .sqrt()
                    .map_err(tol_err)?
                    .to_f64();
                Ok(Self::new(
                    kd,
                    Body::Inexact {
                        nominal: nom,
                        lo: iv.lo,
                        hi: iv.hi,
                        root: RootOp::Sqrt,
                    },
                ))
            }
            Body::Inexact {
                nominal,
                lo,
                hi,
                root,
            } => {
                if *lo < 0.0 {
                    return Err(QErr::plain("argument outside the domain of sqrt"));
                }
                Ok(Self::new(
                    kd,
                    Body::Inexact {
                        nominal: nominal.sqrt(),
                        lo: lo.sqrt(),
                        hi: hi.sqrt(),
                        root: *root,
                    },
                ))
            }
        }
    }

    fn untoleranced(&self, pick: Pick) -> Result<Self, QErr> {
        let dim = &self.kd.dim;
        let body = match (&self.body, pick) {
            (Body::Exact { nominal, .. }, Pick::Nominal) => Body::constant(dim, nominal.clone()),
            (Body::Exact { aff, .. }, Pick::Min) => {
                Body::constant(dim, aff.bounds().map_err(tol_err)?.0)
            }
            (Body::Exact { aff, .. }, Pick::Max) => {
                Body::constant(dim, aff.bounds().map_err(tol_err)?.1)
            }
            (Body::Exact { aff, .. }, Pick::Mid) => {
                let (lo, hi) = aff.bounds().map_err(tol_err)?;
                let half = Magnitude::new(Rational::new(1, 2).map_err(tol_err)?, 0);
                match lo.add(&hi) {
                    Sum::Exact(s) => Body::constant(dim, s.mul(&half)),
                    Sum::Inexact(f) => Body::point(f / 2.0, RootOp::MixedPiSum),
                }
            }
            (Body::Inexact { nominal, root, .. }, Pick::Nominal) => Body::point(*nominal, *root),
            (Body::Inexact { lo, root, .. }, Pick::Min) => Body::point(*lo, *root),
            (Body::Inexact { hi, root, .. }, Pick::Max) => Body::point(*hi, *root),
            (Body::Inexact { lo, hi, root, .. }, Pick::Mid) => Body::point((lo + hi) / 2.0, *root),
        };
        Ok(Self::new(self.kd.clone(), body))
    }

    fn limit(&self, material: Material) -> Result<Self, QErr> {
        let hole = match self.role.as_deref() {
            Some("hole") => true,
            Some("shaft") => false,
            _ => {
                return Err(QErr::new(
                    Some("E010"),
                    "E010 mmc/lmc without a role: call with_role(\"hole\" or \"shaft\") first",
                ));
            }
        };
        // Hole: MMC is the smallest limit; shaft: MMC is the largest.
        let low = (material == Material::Max) == hole;
        self.untoleranced(if low { Pick::Min } else { Pick::Max })
    }

    fn unit_factor(&self, unit: &str) -> Result<Magnitude, QErr> {
        let def = UNITS.parse(unit).map_err(tol_err)?;
        if UNITS.lookup(unit).is_some_and(|d| d.affine) {
            return Err(QErr::plain(format!(
                "affine unit `{unit}` is not supported on Quantity"
            )));
        }
        if def.dim != self.kd.dim {
            return Err(QErr::new(
                Some("Q002"),
                format!("Q002 dimension-mismatch: unit `{unit}` does not match the quantity"),
            ));
        }
        Ok(def.factor)
    }
}

fn exact_binary(
    kd: &KindedDim,
    op: Op,
    a: (&Magnitude, &Affine),
    b: (&Magnitude, &Affine),
) -> Result<Option<Body>, QErr> {
    let _ = kd;
    let nominal = match op {
        Op::Add => match a.0.add(b.0) {
            Sum::Exact(m) => m,
            Sum::Inexact(_) => return Ok(None),
        },
        Op::Sub => match a.0.sub(b.0) {
            Sum::Exact(m) => m,
            Sum::Inexact(_) => return Ok(None),
        },
        Op::Mul => a.0.mul(b.0),
        Op::Div => a.0.div(b.0).map_err(tol_err)?,
    };
    let aff = match op {
        Op::Add => a.1.add(b.1),
        Op::Sub => a.1.sub(b.1),
        Op::Mul => cadetta_units::square::mul_checked(a.1, b.1),
        Op::Div => a.1.div(b.1),
    };
    match aff {
        Ok(aff) => Ok(Some(Body::Exact { nominal, aff })),
        Err(cadetta_units::ToleranceError::MixedPiExponent) => Ok(None),
        Err(e) => Err(tol_err(e)),
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Pick {
    Nominal,
    Min,
    Max,
    Mid,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Material {
    Max,
    Least,
}

fn parse_number(text: &str) -> Result<Rational, QErr> {
    let bad =
        |e: &dyn std::fmt::Display| QErr::new(Some("E001"), format!("bad number `{text}`: {e}"));
    if let Some((head, den)) = text.split_once('/') {
        let den = Rational::parse_decimal(den).map_err(|e| bad(&e))?;
        let (whole, num) = match head.split_once('-') {
            Some((w, n)) if !w.is_empty() => (Rational::parse_decimal(w).map_err(|e| bad(&e))?, n),
            _ => (Rational::from_int(0), head),
        };
        let num = Rational::parse_decimal(num).map_err(|e| bad(&e))?;
        let frac = num.div(&den).map_err(|e| bad(&e))?;
        return Ok(whole.add(&frac));
    }
    Rational::parse_decimal(text).map_err(|e| bad(&e))
}

fn literal(segments: &[cadetta_syntax::lexer::Segment]) -> Result<Quantity, QErr> {
    let mut total: Option<(KindedDim, Magnitude)> = None;
    for seg in segments {
        let unit = UNITS.parse(&seg.unit).map_err(tol_err)?;
        if UNITS.lookup(&seg.unit).is_some_and(|d| d.affine) {
            return Err(QErr::plain(format!(
                "affine unit `{}` is not supported on Quantity",
                seg.unit
            )));
        }
        let value = Magnitude::new(parse_number(&seg.number)?, 0).mul(&unit.factor);
        let kd = KINDS.from_unit(&unit);
        total = Some(match total {
            None => (kd, value),
            Some((first, acc)) => {
                let kd = KINDS.combine(&first, &kd)?.value;
                match acc.add(&value) {
                    Sum::Exact(m) => (kd, m),
                    Sum::Inexact(_) => {
                        return Err(QErr::plain("mixed pi exponents in a compound literal"));
                    }
                }
            }
        });
    }
    let (kd, m) = total.ok_or_else(|| QErr::plain("empty quantity literal"))?;
    let body = Body::constant(&kd.dim, m);
    Ok(Quantity::new(kd, body))
}

fn exact_part(q: &Quantity) -> Result<(&Dim, &Magnitude), QErr> {
    match &q.body {
        Body::Exact { nominal, aff } if aff.is_exact() => Ok((&q.kd.dim, nominal)),
        _ => Err(QErr::plain(
            "a tolerance deviation must be an exact untoleranced value",
        )),
    }
}

fn eval(e: &Expr) -> Result<Quantity, QErr> {
    match &e.kind {
        ExprKind::Number(n) => {
            let kd = KindedDim::anonymous(Dim::default());
            let body = Body::constant(&kd.dim, Magnitude::new(parse_number(n)?, 0));
            Ok(Quantity::new(kd, body))
        }
        ExprKind::Quantity(segs) => literal(segs),
        ExprKind::Paren(inner) => eval(inner),
        ExprKind::Unary(UnOp::Neg, inner) => Ok(eval(inner)?.negated()),
        ExprKind::Binary(op, l, r) => match op {
            BinOp::Add => eval(l)?.binary(&eval(r)?, Op::Add),
            BinOp::Sub => eval(l)?.binary(&eval(r)?, Op::Sub),
            BinOp::Mul => eval(l)?.binary(&eval(r)?, Op::Mul),
            BinOp::Div => eval(l)?.binary(&eval(r)?, Op::Div),
            BinOp::Pow => eval(l)?.power(int_exponent(r)?),
            _ => Err(QErr::plain(
                "operator not supported in a standalone quantity",
            )),
        },
        ExprKind::Tolerance(base, terms) => {
            let base = eval(base)?;
            let (dim, nominal) = match &base.body {
                Body::Exact { nominal, aff } if aff.is_exact() => {
                    (base.kd.dim.clone(), nominal.clone())
                }
                _ => return Err(QErr::plain("tolerance needs an exact untoleranced value")),
            };
            let mut lower = (dim.clone(), zero());
            let mut upper = (dim.clone(), zero());
            for t in terms {
                let dev = eval(&t.value)?;
                let (ddim, mag) = exact_part(&dev)?;
                let (ddim, mag) = (ddim.clone(), mag.clone());
                match t.sign {
                    TolSign::Plus => upper = (ddim, mag),
                    TolSign::Minus => lower = (ddim, mag.neg()),
                    TolSign::PlusMinus => {
                        lower = (ddim.clone(), mag.neg());
                        upper = (ddim, mag);
                    }
                }
            }
            // A bare `0` deviation is dimensionless; adopt the nominal's dimension.
            for d in [&mut lower, &mut upper] {
                if d.1.is_zero() {
                    d.0 = dim.clone();
                }
            }
            let tol =
                Toleranced::from_deviations(dim, nominal.clone(), lower, upper).map_err(tol_err)?;
            let aff = tol.to_affine(fresh_var()).map_err(tol_err)?;
            Ok(Quantity::new(base.kd, Body::Exact { nominal, aff }))
        }
        ExprKind::IsoClass(..) => Err(QErr::plain("ISO tolerance classes are not supported yet")),
        _ => Err(QErr::plain(
            "expression form not supported in a standalone quantity",
        )),
    }
}

fn int_exponent(e: &Expr) -> Result<i32, QErr> {
    let q = eval(e)?;
    let bad = || QErr::plain("exponent must be an integer literal");
    match &q.body {
        Body::Exact { nominal, aff } if aff.is_exact() && nominal.pi_exp() == 0 => {
            let r = nominal.rational();
            if r.denom() != &1.into() {
                return Err(bad());
            }
            r.numer().to_string().parse().map_err(|_| bad())
        }
        _ => Err(bad()),
    }
}

fn parse_source(text: &str) -> Result<Quantity, QErr> {
    let prefix = "model Q {\n__q = ";
    let src = format!("{prefix}{text}\n}}\n");
    let parsed = cadetta_syntax::parser::parse(&src);
    if let Some(d) = parsed.diags.first() {
        return Err(QErr::new(Some(d.code.code()), d.message.clone()));
    }
    let member = parsed
        .file
        .components()
        .next()
        .and_then(|c| c.members.first())
        .ok_or_else(|| QErr::new(Some("E001"), "not a quantity expression"))?;
    let MemberKind::Param(p) = &member.kind else {
        return Err(QErr::new(Some("E001"), "not a quantity expression"));
    };
    if p.value.span.start != prefix.len() || p.value.span.end != prefix.len() + text.len() {
        return Err(QErr::new(
            Some("E001"),
            "trailing text after the quantity expression",
        ));
    }
    eval(&p.value)
}

/// An IR `Exact` object (`{"r": "num/den", "pi": k}`) as a magnitude.
fn ir_exact(v: &serde_json::Value) -> Result<Magnitude, QErr> {
    let bad = || QErr::plain(format!("malformed IR exact magnitude: {v}"));
    let r = v.get("r").and_then(|r| r.as_str()).ok_or_else(bad)?;
    let pi = v.get("pi").and_then(|p| p.as_i64()).ok_or_else(bad)?;
    let (num, den) = r.split_once('/').ok_or_else(bad)?;
    let num = Rational::parse_decimal(num).map_err(|_| bad())?;
    let den = Rational::parse_decimal(den).map_err(|_| bad())?;
    let ratio = num.div(&den).map_err(|_| bad())?;
    Ok(Magnitude::new(ratio, i32::try_from(pi).map_err(|_| bad())?))
}

/// The dimension of an IR `Dim` object.
fn ir_dim(v: &serde_json::Value, angle: i32) -> Result<Dim, QErr> {
    let bad = || QErr::plain(format!("malformed IR dim: {v}"));
    let exp = |key: &str| -> Result<i32, QErr> {
        let n = v.get(key).and_then(|n| n.as_i64()).ok_or_else(bad)?;
        i32::try_from(n).map_err(|_| bad())
    };
    // Order of `Dimension::from_si`: L, M, T, I, Theta, N, J.
    let mut dimension = Dimension::from_si([
        exp("m")?,
        exp("kg")?,
        exp("s")?,
        exp("A")?,
        exp("K")?,
        exp("mol")?,
        exp("cd")?,
    ]);
    if let Some(cur) = v.get("currency").and_then(|c| c.as_object()) {
        for (code, n) in cur {
            let n = n
                .as_i64()
                .and_then(|n| i32::try_from(n).ok())
                .ok_or_else(bad)?;
            dimension = dimension.mul(&Dimension::currency(code).powi(n));
        }
    }
    Ok(Dim::new(dimension, angle))
}

/// The unique built-in kind with this dimension, else anonymous.
fn kind_of_dim(dim: &Dim) -> KindedDim {
    let mut matches = KINDS.all().into_iter().filter(|(_, d)| *d == dim);
    match (matches.next(), matches.next()) {
        (Some((name, _)), None) => KINDS
            .named(name)
            .unwrap_or_else(|_| KindedDim::anonymous(dim.clone())),
        _ => KindedDim::anonymous(dim.clone()),
    }
}

/// Build a Quantity from an IR `Value` object (ir.md 3), tolerance included.
fn from_ir_value(v: &serde_json::Value) -> Result<Quantity, QErr> {
    let bad = |what: &str| QErr::plain(format!("malformed IR value: {what}"));
    let angle = v
        .get("angle")
        .and_then(|a| a.as_i64())
        .and_then(|a| i32::try_from(a).ok())
        .ok_or_else(|| bad("angle"))?;
    let dim = ir_dim(v.get("dim").ok_or_else(|| bad("dim"))?, angle)?;
    let kd = kind_of_dim(&dim);
    let nominal = v.get("nominal").ok_or_else(|| bad("nominal"))?;
    let terms = v
        .get("tol")
        .and_then(|t| t.get("terms"))
        .and_then(|t| t.as_array());
    if let Some(f) = nominal.get("f64").and_then(|f| f.as_f64()) {
        let root = RootOp::MixedPiSum;
        let (mut lo, mut hi) = (f, f);
        for t in terms.into_iter().flatten() {
            lo += ir_exact(t.get("lo").ok_or_else(|| bad("tol lo"))?)?.to_f64();
            hi += ir_exact(t.get("hi").ok_or_else(|| bad("tol hi"))?)?.to_f64();
        }
        debug!("quantity: from IR (inexact)");
        return Ok(Quantity::new(
            kd,
            Body::Inexact {
                nominal: f,
                lo,
                hi,
                root,
            },
        ));
    }
    let nominal = ir_exact(nominal)?;
    let mut aff = Affine::constant(dim.clone(), nominal.clone());
    let mut vars: std::collections::HashMap<String, VarId> = std::collections::HashMap::new();
    for t in terms.into_iter().flatten() {
        let name = t
            .get("var")
            .and_then(|n| n.as_str())
            .ok_or_else(|| bad("tol var"))?;
        let lo = ir_exact(t.get("lo").ok_or_else(|| bad("tol lo"))?)?;
        let hi = ir_exact(t.get("hi").ok_or_else(|| bad("tol hi"))?)?;
        let var = *vars.entry(name.to_string()).or_insert_with(fresh_var);
        let tol =
            Toleranced::from_deviations(dim.clone(), zero(), (dim.clone(), lo), (dim.clone(), hi))
                .map_err(tol_err)?;
        aff = aff
            .add(&tol.to_affine(var).map_err(tol_err)?)
            .map_err(tol_err)?;
    }
    debug!(terms = vars.len(), "quantity: from IR (exact)");
    Ok(Quantity::new(kd, Body::Exact { nominal, aff }))
}

#[pymethods]
impl Quantity {
    /// Build from an IR `Value` JSON object; raises `QuantityError` when malformed.
    #[staticmethod]
    fn _from_ir(text: &str) -> PyResult<Self> {
        let v: serde_json::Value =
            serde_json::from_str(text).map_err(|e| QErr::plain(format!("bad IR JSON: {e}")))?;
        Ok(from_ir_value(&v)?)
    }

    /// Parse source syntax such as `12mm [+-0.1mm]`; raises `QuantityError`.
    #[staticmethod]
    fn _parse(text: &str) -> PyResult<Self> {
        debug!(text, "quantity: parse");
        Ok(parse_source(text)?)
    }

    /// True when the magnitude is exact (not marked inexact).
    #[getter]
    fn exact(&self) -> bool {
        matches!(self.body, Body::Exact { .. })
    }

    /// The kind name, or None for an anonymous compound kind.
    #[getter]
    fn kind(&self) -> Option<String> {
        self.kd.name().map(str::to_string)
    }

    /// SI base-dimension exponents (7 entries, in SI order).
    #[getter]
    fn dim(&self) -> Vec<i32> {
        self.kd.dim.dimension.si().to_vec()
    }

    /// The angle exponent.
    #[getter]
    fn angle(&self) -> i32 {
        self.kd.dim.angle
    }

    /// The role (`"hole"`, `"shaft"`) used by mmc/lmc, if set.
    #[getter]
    fn role(&self) -> Option<String> {
        self.role.clone()
    }

    /// The nominal as an untoleranced quantity.
    #[getter]
    fn nominal(&self) -> PyResult<Self> {
        Ok(self.untoleranced(Pick::Nominal)?)
    }

    /// The lower limit as an untoleranced quantity.
    #[getter]
    fn min(&self) -> PyResult<Self> {
        Ok(self.untoleranced(Pick::Min)?)
    }

    /// The upper limit as an untoleranced quantity.
    #[getter]
    fn max(&self) -> PyResult<Self> {
        Ok(self.untoleranced(Pick::Max)?)
    }

    /// The midpoint of min and max as an untoleranced quantity.
    #[getter]
    fn mid(&self) -> PyResult<Self> {
        Ok(self.untoleranced(Pick::Mid)?)
    }

    /// Maximum-material limit; needs a role (E010 otherwise).
    #[getter]
    fn mmc(&self) -> PyResult<Self> {
        Ok(self.limit(Material::Max)?)
    }

    /// Least-material limit; needs a role (E010 otherwise).
    #[getter]
    fn lmc(&self) -> PyResult<Self> {
        Ok(self.limit(Material::Least)?)
    }

    /// A copy tagged with a mmc/lmc role, `"hole"` or `"shaft"`.
    fn with_role(&self, role: &str) -> PyResult<Self> {
        if role != "hole" && role != "shaft" {
            return Err(
                QErr::plain(format!("role must be \"hole\" or \"shaft\", got `{role}`")).into(),
            );
        }
        let mut q = self.clone();
        q.role = Some(role.to_string());
        Ok(q)
    }

    /// Mirror `as`: reinterpret as another kind of the same dimension.
    fn as_kind(&self, kind: &str) -> PyResult<Self> {
        let kd = KINDS.cast(&self.kd, kind).map_err(QErr::from)?;
        Ok(Self {
            kd,
            body: self.body.clone(),
            role: self.role.clone(),
        })
    }

    /// Nominal in `unit` as a `Fraction`, or None when inexact or when a
    /// factor of pi remains.
    fn to_exact(&self, py: Python<'_>, unit: &str) -> PyResult<Option<Py<PyAny>>> {
        let factor = self.unit_factor(unit)?;
        let Body::Exact { nominal, .. } = &self.body else {
            return Ok(None);
        };
        let v = nominal.div(&factor).map_err(tol_err)?;
        if v.pi_exp() != 0 {
            return Ok(None);
        }
        let text = format!("{}/{}", v.rational().numer(), v.rational().denom());
        let fraction = py
            .import("fractions")?
            .getattr("Fraction")?
            .call1((text,))?;
        Ok(Some(fraction.unbind()))
    }

    /// Nominal in `unit` as a float; always works.
    fn to_float(&self, unit: &str) -> PyResult<f64> {
        let factor = self.unit_factor(unit)?;
        Ok(match &self.body {
            Body::Exact { nominal, .. } => nominal.div(&factor).map_err(tol_err)?.to_f64(),
            Body::Inexact { nominal, .. } => nominal / factor.to_f64(),
        })
    }

    fn __add__(&self, py: Python<'_>, other: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        self.dispatch(py, other, Op::Add, false)
    }

    fn __radd__(&self, py: Python<'_>, other: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        self.dispatch(py, other, Op::Add, true)
    }

    fn __sub__(&self, py: Python<'_>, other: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        self.dispatch(py, other, Op::Sub, false)
    }

    fn __rsub__(&self, py: Python<'_>, other: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        self.dispatch(py, other, Op::Sub, true)
    }

    fn __mul__(&self, py: Python<'_>, other: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        self.dispatch(py, other, Op::Mul, false)
    }

    fn __rmul__(&self, py: Python<'_>, other: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        self.dispatch(py, other, Op::Mul, true)
    }

    fn __truediv__(&self, py: Python<'_>, other: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        self.dispatch(py, other, Op::Div, false)
    }

    fn __rtruediv__(&self, py: Python<'_>, other: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        self.dispatch(py, other, Op::Div, true)
    }

    fn __neg__(&self) -> Self {
        self.negated()
    }

    fn __pow__(&self, exp: i32, _modulo: Option<&Bound<'_, PyAny>>) -> PyResult<Self> {
        Ok(self.power(exp)?)
    }

    /// Square root with the root-of-dimension rule (Q011).
    fn sqrt(&self) -> PyResult<Self> {
        Ok(self.square_root()?)
    }

    fn __repr__(&self) -> String {
        let (n, lo, hi, _) = match self.body.interval() {
            Ok(t) => t,
            Err(_) => (f64::NAN, f64::NAN, f64::NAN, None),
        };
        format!(
            "Quantity(kind={:?}, si_nominal={n}, si_min={lo}, si_max={hi}, exact={})",
            self.kd.name().unwrap_or("anonymous"),
            self.exact()
        )
    }
}

impl Quantity {
    fn dispatch(
        &self,
        py: Python<'_>,
        other: &Bound<'_, PyAny>,
        op: Op,
        reflected: bool,
    ) -> PyResult<Py<PyAny>> {
        let rhs = if let Ok(q) = other.extract::<PyRef<'_, Quantity>>() {
            q.clone()
        } else if let Ok(n) = other.extract::<i64>()
            && !other.is_instance_of::<pyo3::types::PyBool>()
        {
            Self::scalar(n)
        } else {
            return Ok(PyNotImplemented::get(py).to_owned().into_any().unbind());
        };
        let out = if reflected {
            rhs.binary(self, op)
        } else {
            self.binary(&rhs, op)
        }?;
        Ok(Py::new(py, out)?.into_any())
    }
}

/// Register `Quantity` and `QuantityError` on the extension module.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Quantity>()?;
    m.add("QuantityError", m.py().get_type::<QuantityError>())?;
    Ok(())
}
