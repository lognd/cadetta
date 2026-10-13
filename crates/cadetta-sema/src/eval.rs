//! Evaluation of parameters and `let`s to exact values (quantities.md 5.3, 5.4).
//!
//! Every parameter and member `let` of every component without parameters is
//! evaluated on demand to a [`Quant`]: an affine tolerance form plus the
//! tracked exact nominal of the expression. A parameter stores its *design*
//! tolerance only (nominal plus the tolerance written on it, or its referent's
//! source variable when it is a pure reference); a `let` keeps the full
//! analysis form of its expression and is flagged as analysis.
//!
//! Decisions made here (written into quantities.md 5.3, 5.4 and 5.5): the
//! nominal of an expression is tracked separately from the affine centre, so
//! `head_dia + 1mm` stores the nominal sum even when `head_dia` is asymmetric;
//! a bracket with one `+v` or `-v` term has `0` on the other side; a tolerance
//! written inside a larger expression or a `let` owns an anonymous source
//! variable; failures with no spec code carry the internal code `EVAL`.
//! Parameterized components are evaluated only as instances (a later ticket),
//! and inexact functions (`sqrt`, trigonometry) are deferred to the standard
//! functions ticket.

// frob:ticket 01M4SPWPPPSEEFAHYTY5H3YD54

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use cadetta_syntax::ast::{BinOp, Component, Expr, ExprKind, MemberKind, TolSign, TolTerm, UnOp};
use cadetta_syntax::lexer::Segment;
use cadetta_syntax::span::Span;
use cadetta_units::inexact::round_value;
use cadetta_units::square::{mul_checked, square};
use cadetta_units::{
    Affine, Dim, InexactError, Magnitude, Rational, Rounding, Sum, Temperature, TemperatureUnit,
    ToleranceError, Toleranced, UnitTable, Value, VarId,
};
use tracing::{debug, trace, warn};

use crate::deps::Severity;
use crate::resolve::{CanonPath, Ctx, Level, MemberSeg, PackageSrc, Resolved, Resolver};

/// Accessors that read a quantity's bounds (quantities.md 5.2).
const ACCESSORS: &[&str] = &["nominal", "min", "max", "mid", "mmc", "lmc"];

/// Largest integer exponent `x ^ n` evaluates by repeated multiplication.
const MAX_POWER: i32 = 64;

/// An evaluated quantity: its affine form and the exact nominal of its expression.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Quant {
    /// The affine tolerance form over the source variables.
    pub form: Affine,
    /// The nominal tracked through arithmetic (quantities.md 5.3).
    pub nominal: Magnitude,
}

/// An evaluated value.
#[derive(Clone, PartialEq, Eq, Debug)]
#[allow(clippy::large_enum_variant)] // quantities dominate; boxing would burden every reader
pub enum Val {
    /// A quantity (a bare number is a dimensionless quantity).
    Quantity(Quant),
    /// A boolean.
    Bool(bool),
    /// A string.
    Text(String),
}

/// A parameter or `let` after evaluation.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Evaluated {
    /// The value.
    pub value: Val,
    /// True for a member `let` (an analysis value, never a source variable).
    pub analysis: bool,
    /// The source variable this parameter owns (or aliases), if toleranced.
    pub var: Option<VarId>,
    /// The referent when the parameter is a pure reference (an alias).
    pub alias_of: Option<CanonPath>,
    /// The definition's span.
    pub span: Span,
}

/// A finding made while evaluating.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct EvalDiagnostic {
    /// The lint code (`Q002`, `Q010`, `E002`, `E010`, or the internal `EVAL`).
    pub code: &'static str,
    /// Error or warning.
    pub severity: Severity,
    /// The human message.
    pub message: String,
    /// The primary span.
    pub span: Span,
}

impl fmt::Display for EvalDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

/// The result of evaluating a project.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Evaluation {
    /// Every successfully evaluated member, by canonical path.
    pub values: BTreeMap<CanonPath, Evaluated>,
    /// Everything that went wrong, in evaluation order.
    pub diagnostics: Vec<EvalDiagnostic>,
}

impl Evaluation {
    /// The evaluated member at `path`, if it evaluated.
    pub fn get(&self, path: &CanonPath) -> Option<&Evaluated> {
        self.values.get(path)
    }
}

/// Why an expression produced no value.
enum Fail {
    /// Already reported elsewhere (a cycle is D003; a dependency failed).
    Silent,
    /// A new diagnostic.
    Diag(EvalDiagnostic),
}

impl From<EvalDiagnostic> for Fail {
    fn from(d: EvalDiagnostic) -> Self {
        Self::Diag(d)
    }
}

type R<T> = Result<T, Fail>;

fn diag(code: &'static str, span: Span, message: impl Into<String>) -> Fail {
    let message = message.into();
    warn!(code, %message, "evaluation error");
    Fail::Diag(EvalDiagnostic {
        code,
        severity: Severity::Error,
        message,
        span,
    })
}

fn tol_diag(e: &ToleranceError, span: Span) -> Fail {
    match e {
        ToleranceError::DimensionMismatch { .. } => diag("Q002", span, e.to_string()),
        _ => diag("EVAL", span, e.to_string()),
    }
}

fn exact(sum: Sum, span: Span) -> R<Magnitude> {
    match sum {
        Sum::Exact(m) => Ok(m),
        Sum::Inexact(_) => Err(diag(
            "EVAL",
            span,
            "sum of different powers of pi is inexact and not representable here",
        )),
    }
}

fn half(m: &Magnitude, span: Span) -> R<Magnitude> {
    let two = Magnitude::new(Rational::from_int(2), 0);
    m.div(&two).map_err(|e| diag("EVAL", span, e.to_string()))
}

/// Sign of `a - b` (-1, 0, 1), decided on the exact difference.
fn cmp_mag(a: &Magnitude, b: &Magnitude) -> std::cmp::Ordering {
    let d = match a.sub(b) {
        Sum::Exact(m) => m.to_f64(),
        Sum::Inexact(f) => f,
    };
    d.partial_cmp(&0.0).unwrap_or(std::cmp::Ordering::Equal)
}

impl Quant {
    /// An untoleranced quantity.
    pub fn constant(dim: Dim, value: Magnitude) -> Self {
        Self {
            form: Affine::constant(dim, value.clone()),
            nominal: value,
        }
    }

    /// The dimension.
    pub fn dim(&self) -> &Dim {
        self.form.dim()
    }

    fn add(&self, o: &Self, span: Span) -> R<Self> {
        let form = self.form.add(&o.form).map_err(|e| tol_diag(&e, span))?;
        let nominal = exact(self.nominal.add(&o.nominal), span)?;
        Ok(Self { form, nominal })
    }

    fn sub(&self, o: &Self, span: Span) -> R<Self> {
        let form = self.form.sub(&o.form).map_err(|e| tol_diag(&e, span))?;
        let nominal = exact(self.nominal.sub(&o.nominal), span)?;
        Ok(Self { form, nominal })
    }

    fn mul(&self, o: &Self, span: Span) -> R<Self> {
        let form = mul_checked(&self.form, &o.form).map_err(|e| tol_diag(&e, span))?;
        Ok(Self {
            form,
            nominal: self.nominal.mul(&o.nominal),
        })
    }

    fn div(&self, o: &Self, span: Span) -> R<Self> {
        let form = self.form.div(&o.form).map_err(|e| tol_diag(&e, span))?;
        let nominal = self
            .nominal
            .div(&o.nominal)
            .map_err(|e| diag("EVAL", span, e.to_string()))?;
        Ok(Self { form, nominal })
    }

    fn neg(&self) -> Self {
        Self {
            form: self.form.neg(),
            nominal: self.nominal.neg(),
        }
    }

    fn pow(&self, n: i32, span: Span) -> R<Self> {
        if n == 0 {
            return Ok(Self::constant(
                Dim::default(),
                Magnitude::new(Rational::one(), 0),
            ));
        }
        if n.abs() > MAX_POWER {
            return Err(diag("EVAL", span, format!("exponent {n} is too large")));
        }
        let positive = if n.abs() == 2 {
            Self {
                form: square(&self.form).map_err(|e| tol_diag(&e, span))?,
                nominal: self.nominal.mul(&self.nominal),
            }
        } else {
            let mut acc = self.clone();
            for _ in 1..n.abs() {
                acc = acc.mul(self, span)?;
            }
            acc
        };
        if n > 0 {
            Ok(positive)
        } else {
            let one = Self::constant(Dim::default(), Magnitude::new(Rational::one(), 0));
            one.div(&positive, span)
        }
    }

    /// The accessor `name` of this quantity as an untoleranced constant.
    fn accessor(&self, name: &str, span: Span) -> R<Self> {
        let dim = self.dim().clone();
        let m = match name {
            "nominal" => self.nominal.clone(),
            "min" | "max" | "mid" => {
                let (lo, hi) = self.form.bounds().map_err(|e| tol_diag(&e, span))?;
                match name {
                    "min" => lo,
                    "max" => hi,
                    _ => half(&exact(lo.add(&hi), span)?, span)?,
                }
            }
            _ => {
                return Err(diag(
                    "E010",
                    span,
                    format!("`.{name}` needs a role (hole or boss) to say which side is material"),
                ));
            }
        };
        Ok(Self::constant(dim, m))
    }
}

/// How a reference expression resolved.
enum RefKind {
    /// A member of a definition (or the implicit instance), with an optional accessor.
    Member {
        key: CanonPath,
        accessor: Option<String>,
    },
    /// A block-local binding.
    Local(String),
}

/// A member definition indexed for evaluation.
#[derive(Clone)]
struct Def<'p> {
    expr: &'p Expr,
    is_let: bool,
    ctx: Ctx,
    span: Span,
}

/// The names visible while evaluating one definition.
struct Scope {
    ctx: Ctx,
    locals: Vec<(String, Val)>,
}

impl Scope {
    fn push(&mut self, name: &str, v: Val) {
        self.ctx.locals.push(name.to_owned());
        self.locals.push((name.to_owned(), v));
    }

    fn truncate(&mut self, n: usize) {
        self.ctx.locals.truncate(n);
        self.locals.truncate(n);
    }
}

/// Evaluates the parameters and `let`s of a project, on demand and memoised.
pub struct Evaluator<'r, 'p> {
    resolver: &'r Resolver<'p>,
    units: &'r UnitTable,
    defs: BTreeMap<CanonPath, Def<'p>>,
    done: BTreeMap<CanonPath, Option<Evaluated>>,
    in_progress: BTreeSet<CanonPath>,
    next_var: u32,
    diagnostics: Vec<EvalDiagnostic>,
}

impl<'r, 'p> Evaluator<'r, 'p> {
    /// Indexes every parameter and `let` of every component without parameters.
    pub fn new(
        resolver: &'r Resolver<'p>,
        units: &'r UnitTable,
        packages: &[PackageSrc<'p>],
    ) -> Self {
        let mut defs = BTreeMap::new();
        for p in packages {
            for m in &p.modules {
                for c in m.file.components() {
                    index_component(&mut defs, &p.name, &m.path, c);
                }
            }
        }
        debug!(members = defs.len(), "evaluator: indexed members");
        Self {
            resolver,
            units,
            defs,
            done: BTreeMap::new(),
            in_progress: BTreeSet::new(),
            next_var: 0,
            diagnostics: Vec::new(),
        }
    }

    /// Evaluates every indexed member and returns the values and diagnostics.
    pub fn evaluate(mut self) -> Evaluation {
        let keys: Vec<CanonPath> = self.defs.keys().cloned().collect();
        for k in &keys {
            self.value_of(k);
        }
        let values = self
            .done
            .into_iter()
            .filter_map(|(k, v)| v.map(|v| (k, v)))
            .collect();
        debug!(
            diagnostics = self.diagnostics.len(),
            "evaluator: evaluation finished"
        );
        Evaluation {
            values,
            diagnostics: self.diagnostics,
        }
    }

    fn fresh_var(&mut self) -> VarId {
        let v = VarId(self.next_var);
        self.next_var += 1;
        v
    }

    /// The memoised value of a member; `None` when it failed or is cyclic.
    fn value_of(&mut self, key: &CanonPath) -> Option<Evaluated> {
        if let Some(v) = self.done.get(key) {
            return v.clone();
        }
        let def = self.defs.get(key)?.clone();
        if !self.in_progress.insert(key.clone()) {
            trace!(?key, "evaluator: cycle, left to D003");
            return None;
        }
        let mut scope = Scope {
            ctx: def.ctx.clone(),
            locals: Vec::new(),
        };
        let result = if def.is_let {
            self.eval_let(&mut scope, &def)
        } else {
            self.eval_param(&mut scope, &def, key)
        };
        self.in_progress.remove(key);
        let out = match result {
            Ok(v) => Some(v),
            Err(Fail::Silent) => None,
            Err(Fail::Diag(d)) => {
                self.diagnostics.push(d);
                None
            }
        };
        self.done.insert(key.clone(), out.clone());
        out
    }

    fn eval_let(&mut self, sc: &mut Scope, def: &Def<'p>) -> R<Evaluated> {
        let value = self.eval(sc, def.expr)?;
        trace!("evaluator: let evaluated, analysis form kept");
        Ok(Evaluated {
            value,
            analysis: true,
            var: None,
            alias_of: None,
            span: def.span,
        })
    }

    fn eval_param(&mut self, sc: &mut Scope, def: &Def<'p>, _key: &CanonPath) -> R<Evaluated> {
        let e = strip(def.expr);
        let span = def.span;
        // A pure reference is an alias of its referent's source variable.
        if matches!(e.kind, ExprKind::Path(_) | ExprKind::Field(..))
            && let Ok(RefKind::Member {
                key: rk,
                accessor: None,
            }) = self.classify(sc, e)
            && let Some(ev) = self.value_of(&rk)
            && !ev.analysis
            && matches!(ev.value, Val::Quantity(_))
        {
            debug!(?rk, "evaluator: parameter aliases its referent");
            return Ok(Evaluated {
                value: ev.value,
                analysis: false,
                var: ev.var,
                alias_of: Some(ev.alias_of.unwrap_or(rk)),
                span,
            });
        }
        match &e.kind {
            ExprKind::Tolerance(inner, terms) => {
                let var = self.fresh_var();
                let q = self.tolerance(sc, inner, terms, var, e.span)?;
                Ok(owned(Val::Quantity(q), Some(var), span))
            }
            ExprKind::Binary(BinOp::Range, l, r) => {
                let var = self.fresh_var();
                let q = self.range(sc, l, r, var, e.span)?;
                Ok(owned(Val::Quantity(q), Some(var), span))
            }
            _ => match self.eval(sc, e)? {
                Val::Quantity(q) => {
                    trace!("evaluator: parameter stores the nominal only");
                    let c = Quant::constant(q.dim().clone(), q.nominal);
                    Ok(owned(Val::Quantity(c), None, span))
                }
                other => Ok(owned(other, None, span)),
            },
        }
    }

    fn classify(&self, sc: &Scope, e: &Expr) -> R<RefKind> {
        let resolved = self.resolver.resolve_ref(&sc.ctx, e, None).map_err(|re| {
            Fail::Diag(EvalDiagnostic {
                code: re.code,
                severity: Severity::Error,
                message: re.message,
                span: re.span,
            })
        })?;
        match resolved {
            Resolved::Local(n) => Ok(RefKind::Local(n)),
            Resolved::Member { path, level, .. } => {
                if level == Level::Instance || !path.parts.is_empty() {
                    return Err(diag(
                        "EVAL",
                        e.span,
                        "reading a part instance is evaluated by the parts ticket",
                    ));
                }
                let mut key = path.clone();
                let accessor = match path.members.as_slice() {
                    [MemberSeg::Name(_)] => None,
                    [MemberSeg::Name(_), MemberSeg::Name(a)] if ACCESSORS.contains(&a.as_str()) => {
                        Some(a.clone())
                    }
                    _ => {
                        return Err(diag("EVAL", e.span, "member chain is not evaluable here"));
                    }
                };
                key.members.truncate(1);
                Ok(RefKind::Member { key, accessor })
            }
            other => Err(diag(
                "EVAL",
                e.span,
                format!("reference `{other:?}` is not a value evaluated at this stage"),
            )),
        }
    }

    fn eval_ref(&mut self, sc: &Scope, e: &Expr) -> R<Val> {
        match self.classify(sc, e)? {
            RefKind::Local(n) => sc
                .locals
                .iter()
                .rev()
                .find(|(l, _)| *l == n)
                .map(|(_, v)| v.clone())
                .ok_or_else(|| diag("EVAL", e.span, format!("local `{n}` has no value"))),
            RefKind::Member { key, accessor } => {
                let ev = self.value_of(&key).ok_or(Fail::Silent)?;
                match (ev.value, accessor) {
                    (v, None) => Ok(v),
                    (Val::Quantity(q), Some(a)) => Ok(Val::Quantity(q.accessor(&a, e.span)?)),
                    (_, Some(a)) => Err(diag("EVAL", e.span, format!("`.{a}` needs a quantity"))),
                }
            }
        }
    }

    fn quant(&mut self, sc: &mut Scope, e: &Expr) -> R<Quant> {
        match self.eval(sc, e)? {
            Val::Quantity(q) => Ok(q),
            _ => Err(diag("EVAL", e.span, "a quantity is required here")),
        }
    }

    fn eval(&mut self, sc: &mut Scope, e: &Expr) -> R<Val> {
        let span = e.span;
        match &e.kind {
            ExprKind::Number(t) => {
                let m = parse_number(t, span)?;
                Ok(Val::Quantity(Quant::constant(Dim::default(), m)))
            }
            ExprKind::Quantity(segs) => Ok(Val::Quantity(self.quantity(segs, span)?)),
            ExprKind::Str(s) => Ok(Val::Text(s.clone())),
            ExprKind::Bool(b) => Ok(Val::Bool(*b)),
            ExprKind::Paren(inner) => self.eval(sc, inner),
            ExprKind::As(inner, _) => self.eval(sc, inner),
            ExprKind::Path(_) | ExprKind::Field(..) => self.eval_ref(sc, e),
            ExprKind::Unary(UnOp::Neg, x) => Ok(Val::Quantity(self.quant(sc, x)?.neg())),
            ExprKind::Binary(op, l, r) => self.binary(sc, *op, l, r, span),
            ExprKind::Tolerance(inner, terms) => {
                let var = self.fresh_var();
                Ok(Val::Quantity(self.tolerance(sc, inner, terms, var, span)?))
            }
            ExprKind::Call {
                callee,
                optional: false,
                args,
            } => self.call(sc, callee, args, span),
            ExprKind::Block(b) => {
                let mark = sc.locals.len();
                let mut result = Ok(());
                for l in &b.lets {
                    match self.eval(sc, &l.value) {
                        Ok(v) => sc.push(&l.name.name, v),
                        Err(f) => {
                            result = Err(f);
                            break;
                        }
                    }
                }
                let out = match result {
                    Ok(()) => self.eval(sc, &b.tail),
                    Err(f) => Err(f),
                };
                sc.truncate(mark);
                out
            }
            _ => Err(diag(
                "EVAL",
                span,
                "this construct is not evaluated by this stage",
            )),
        }
    }

    fn binary(&mut self, sc: &mut Scope, op: BinOp, l: &Expr, r: &Expr, span: Span) -> R<Val> {
        match op {
            BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div => {
                let a = self.quant(sc, l)?;
                let b = self.quant(sc, r)?;
                let q = match op {
                    BinOp::Add => a.add(&b, span)?,
                    BinOp::Sub => a.sub(&b, span)?,
                    BinOp::Mul => a.mul(&b, span)?,
                    _ => a.div(&b, span)?,
                };
                Ok(Val::Quantity(q))
            }
            BinOp::Pow => {
                let n = integer_literal(r)
                    .ok_or_else(|| diag("EVAL", span, "the exponent must be an integer literal"))?;
                let a = self.quant(sc, l)?;
                Ok(Val::Quantity(a.pow(n, span)?))
            }
            BinOp::Range => {
                let var = self.fresh_var();
                Ok(Val::Quantity(self.range(sc, l, r, var, span)?))
            }
            _ => Err(diag(
                "EVAL",
                span,
                "this operator is not evaluated by this stage",
            )),
        }
    }

    fn quantity(&self, segs: &[Segment], span: Span) -> R<Quant> {
        let mut acc: Option<Quant> = None;
        for s in segs {
            let n = parse_rational(&s.number, span)?;
            let unit = self
                .units
                .parse(&s.unit)
                .map_err(|e| diag("EVAL", span, e.to_string()))?;
            let m = match TemperatureUnit::parse(&s.unit) {
                Ok(tu) if tu.class() == cadetta_units::TempClass::Absolute => {
                    let t =
                        Temperature::new(&n, tu).map_err(|e| diag("EVAL", span, e.to_string()))?;
                    Magnitude::new(t.kelvin().clone(), 0)
                }
                _ => Magnitude::new(n, 0).mul(&unit.factor),
            };
            let q = Quant::constant(unit.dim, m);
            acc = Some(match acc {
                None => q,
                Some(a) => a.add(&q, span)?,
            });
        }
        acc.ok_or_else(|| diag("EVAL", span, "empty quantity literal"))
    }

    fn tolerance(
        &mut self,
        sc: &mut Scope,
        inner: &Expr,
        terms: &[TolTerm],
        var: VarId,
        span: Span,
    ) -> R<Quant> {
        let base = self.quant(sc, inner)?;
        let dim = base.dim().clone();
        let zero = Magnitude::new(Rational::from_int(0), 0);
        let mut devs: Vec<Magnitude> = Vec::new();
        for t in terms {
            let v = self.quant(sc, &t.value)?;
            let mag = if v.dim().dimension.is_dimensionless() && v.nominal.is_zero() {
                zero.clone()
            } else {
                v.dim()
                    .check_same_dimension(&dim)
                    .map_err(|e| diag("Q002", t.value.span, e.to_string()))?;
                v.nominal
            };
            match t.sign {
                TolSign::Plus => devs.push(mag),
                TolSign::Minus => devs.push(mag.neg()),
                TolSign::PlusMinus => {
                    devs.push(mag.neg());
                    devs.push(mag);
                }
            }
        }
        if devs.len() == 1 {
            devs.push(zero);
        }
        let mut lower = devs[0].clone();
        let mut upper = devs[0].clone();
        for d in &devs[1..] {
            if cmp_mag(d, &lower).is_lt() {
                lower = d.clone();
            }
            if cmp_mag(d, &upper).is_gt() {
                upper = d.clone();
            }
        }
        let t = Toleranced::from_deviations(
            dim.clone(),
            base.nominal.clone(),
            (dim.clone(), lower),
            (dim, upper),
        )
        .map_err(|e| tol_diag(&e, span))?;
        let form = t.to_affine(var).map_err(|e| tol_diag(&e, span))?;
        trace!(?var, "evaluator: tolerance bracket owns a source variable");
        Ok(Quant {
            form,
            nominal: base.nominal,
        })
    }

    fn range(&mut self, sc: &mut Scope, l: &Expr, r: &Expr, var: VarId, span: Span) -> R<Quant> {
        let lo = self.quant(sc, l)?;
        let hi = self.quant(sc, r)?;
        lo.dim()
            .check_same_dimension(hi.dim())
            .map_err(|e| diag("Q002", span, e.to_string()))?;
        let t = Toleranced::from_interval(lo.dim().clone(), lo.nominal, hi.nominal)
            .map_err(|e| tol_diag(&e, span))?;
        let form = t.to_affine(var).map_err(|e| tol_diag(&e, span))?;
        Ok(Quant {
            form,
            nominal: t.nominal().clone(),
        })
    }

    fn call(
        &mut self,
        sc: &mut Scope,
        callee: &Expr,
        args: &[cadetta_syntax::ast::Arg],
        span: Span,
    ) -> R<Val> {
        let name = match &callee.kind {
            ExprKind::Path(p) if p.segments.len() == 1 => p.segments[0].name.as_str(),
            _ => "",
        };
        let mode = match name {
            "round_to" => Rounding::Round,
            "ceil_to" => Rounding::Ceil,
            "floor_to" => Rounding::Floor,
            _ => {
                return Err(diag(
                    "EVAL",
                    span,
                    "this function is not evaluated by this stage",
                ));
            }
        };
        let [x, step] = args else {
            return Err(diag("EVAL", span, format!("`{name}` takes (x, step)")));
        };
        let x = self.quant(sc, &x.value)?;
        let step = self.quant(sc, &step.value)?;
        let rounded = round_value(
            &Value::Exact(x.nominal.clone()),
            x.dim(),
            &step.nominal,
            step.dim(),
            mode,
        )
        .map_err(|e| round_diag(&e, span))?;
        let delta = exact(rounded.sub(&x.nominal), span)?;
        let form = x
            .form
            .add(&Affine::constant(x.dim().clone(), delta))
            .map_err(|e| tol_diag(&e, span))?;
        trace!(name, "evaluator: rounded the nominal, deviations kept");
        Ok(Val::Quantity(Quant {
            form,
            nominal: rounded,
        }))
    }
}

fn round_diag(e: &InexactError, span: Span) -> Fail {
    diag(e.code().unwrap_or("EVAL"), span, e.to_string())
}

fn owned(value: Val, var: Option<VarId>, span: Span) -> Evaluated {
    Evaluated {
        value,
        analysis: false,
        var,
        alias_of: None,
        span,
    }
}

fn strip(mut e: &Expr) -> &Expr {
    while let ExprKind::Paren(inner) = &e.kind {
        e = inner;
    }
    e
}

fn integer_literal(e: &Expr) -> Option<i32> {
    match &strip(e).kind {
        ExprKind::Number(t) if t.bytes().all(|b| b.is_ascii_digit()) => t.parse().ok(),
        ExprKind::Unary(UnOp::Neg, x) => integer_literal(x).map(|n| -n),
        _ => None,
    }
}

fn parse_rational(text: &str, span: Span) -> R<Rational> {
    let bad = |why: String| diag("EVAL", span, format!("number `{text}`: {why}"));
    let dec = |s: &str| Rational::parse_decimal(s).map_err(|e| bad(e.to_string()));
    match text.rsplit_once('/') {
        None => dec(text),
        Some((num, den)) => {
            let den = dec(den)?;
            // `1-1/2` is a mixed number: whole part plus fraction.
            let (whole, num) = match num.split_once('-') {
                Some((w, n)) => (Some(dec(w)?), dec(n)?),
                None => (None, dec(num)?),
            };
            let frac = num.div(&den).map_err(|e| bad(e.to_string()))?;
            Ok(whole.map_or(frac.clone(), |w| w.add(&frac)))
        }
    }
}

fn parse_number(text: &str, span: Span) -> R<Magnitude> {
    Ok(Magnitude::new(parse_rational(text, span)?, 0))
}

fn index_component<'p>(
    defs: &mut BTreeMap<CanonPath, Def<'p>>,
    package: &str,
    module: &[String],
    c: &'p Component,
) {
    if !c.params.is_empty() {
        trace!(component = %c.name.name, "evaluator: parameterized component skipped");
        return;
    }
    let ctx = Ctx {
        package: package.to_owned(),
        module: module.to_vec(),
        component: Some(c.name.name.clone()),
        locals: Vec::new(),
    };
    let item = CanonPath::item(package, module, &c.name.name);
    for m in &c.members {
        let (name, expr, is_let, span) = match &m.kind {
            MemberKind::Param(p) => (&p.name.name, &p.value, false, p.name.span),
            MemberKind::Let(l) => (&l.name.name, &l.value, true, l.span),
            _ => continue,
        };
        let mut key = item.clone();
        key.members.push(MemberSeg::Name(name.clone()));
        defs.insert(
            key,
            Def {
                expr,
                is_let,
                ctx: ctx.clone(),
                span,
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolve::ModuleSrc;
    use cadetta_syntax::ast::File;
    use cadetta_syntax::parser::parse;

    fn file(src: &str) -> File {
        let p = parse(src);
        assert!(p.diags.is_empty(), "{:?}", p.diags);
        p.file
    }

    fn run(f: &File) -> Evaluation {
        let pkgs = vec![PackageSrc {
            name: "p".to_owned(),
            deps: Vec::new(),
            modules: vec![ModuleSrc {
                path: Vec::new(),
                file: f,
                enums: &[],
            }],
        }];
        let (resolver, errs) = Resolver::new(pkgs.clone());
        assert!(errs.is_empty());
        let units = UnitTable::builtin();
        Evaluator::new(&resolver, &units, &pkgs).evaluate()
    }

    fn at(comp: &str, member: &str) -> CanonPath {
        let mut p = CanonPath::item("p", &[], comp);
        p.members.push(MemberSeg::Name(member.to_owned()));
        p
    }

    fn quant(ev: &Evaluation, comp: &str, member: &str) -> Quant {
        match &ev.get(&at(comp, member)).expect("evaluated").value {
            Val::Quantity(q) => q.clone(),
            other => panic!("not a quantity: {other:?}"),
        }
    }

    fn m(n: i64, d: i64) -> Magnitude {
        Magnitude::new(Rational::new(n, d).expect("denominator"), 0)
    }

    // frob:tests round_to_on_exact_input_is_exact
    #[test]
    fn round_to_on_exact_input_is_exact() {
        let f = file(
            "model M {\n  a = round_to(10.3mm, 0.5mm)\n  b = 10.5mm\n  \
             c = ceil_to(10.1mm, 0.5mm)\n  d = floor_to(10.9mm, 0.5mm)\n}\n",
        );
        let ev = run(&f);
        assert!(ev.diagnostics.is_empty(), "{:?}", ev.diagnostics);
        let a = quant(&ev, "M", "a");
        assert!(a.form.is_exact());
        assert_eq!(a, quant(&ev, "M", "b"));
        assert_eq!(quant(&ev, "M", "c").nominal, m(105, 10_000));
        assert_eq!(quant(&ev, "M", "d").nominal, m(105, 10_000));
    }

    // frob:tests exact_sum_of_inch_and_millimetre_is_338_1mm
    #[test]
    fn exact_sum_of_inch_and_millimetre_is_338_1mm() {
        let f = file(
            "model Wheel {\n  hub_width = 0.75in\n}\n\
             model Frame {\n  span = 300mm\n  length = 2 * Wheel.hub_width + Frame.span\n  \
             expected = 338.1mm\n}\n",
        );
        let ev = run(&f);
        assert!(ev.diagnostics.is_empty(), "{:?}", ev.diagnostics);
        assert_eq!(
            quant(&ev, "Frame", "length"),
            quant(&ev, "Frame", "expected")
        );
        assert_eq!(quant(&ev, "Frame", "length").nominal, m(3381, 10_000));
    }

    // frob:tests let_keeps_analysis_form_and_is_readable_elsewhere
    #[test]
    fn let_keeps_analysis_form_and_is_readable_elsewhere() {
        let f = file(
            "model A {\n  t = 5mm [+-0.1mm]\n  let stack = t * 2 + t\n}\n\
             model B {\n  seen = A.stack\n  other = A.t\n}\n",
        );
        let ev = run(&f);
        assert!(ev.diagnostics.is_empty(), "{:?}", ev.diagnostics);
        let l = ev.get(&at("A", "stack")).expect("let");
        assert!(l.analysis);
        assert!(l.var.is_none());
        let Val::Quantity(q) = &l.value else { panic!() };
        // t appears three times: 15mm +- 0.3mm, repeated terms counted once.
        assert_eq!(q.form.bounds().unwrap(), (m(147, 10_000), m(153, 10_000)));
        assert!(!ev.get(&at("A", "t")).unwrap().analysis);
        // A parameter reading a let stores its nominal only and is no alias.
        let seen = ev.get(&at("B", "seen")).unwrap();
        assert!(!seen.analysis && seen.alias_of.is_none() && seen.var.is_none());
        assert_eq!(quant(&ev, "B", "seen").nominal, m(15, 1000));
        assert!(ev.get(&at("B", "other")).unwrap().alias_of.is_some());
    }

    // frob:tests parameter_from_arithmetic_stores_nominal_and_own_tolerance
    #[test]
    fn parameter_from_arithmetic_stores_nominal_and_own_tolerance() {
        let f = file(
            "model M {\n  t = 5mm [+-0.1mm]\n  plain = t * 2\n  \
             own = (t * 2) [+-0.05mm]\n  let all = t * 2\n}\n",
        );
        let ev = run(&f);
        assert!(ev.diagnostics.is_empty(), "{:?}", ev.diagnostics);
        let plain = quant(&ev, "M", "plain");
        assert_eq!(plain.form.bounds().unwrap(), (m(1, 100), m(1, 100)));
        assert!(ev.get(&at("M", "plain")).unwrap().var.is_none());
        let own = quant(&ev, "M", "own");
        assert_eq!(
            own.form.bounds().unwrap(),
            (m(995, 100_000), m(1005, 100_000))
        );
        let own_ev = ev.get(&at("M", "own")).unwrap();
        assert!(own_ev.var.is_some() && own_ev.var != ev.get(&at("M", "t")).unwrap().var);
        let all = quant(&ev, "M", "all");
        assert_eq!(all.form.bounds().unwrap(), (m(98, 10_000), m(102, 10_000)));
    }

    // frob:tests pure_reference_parameter_aliases_referent_variable
    #[test]
    fn pure_reference_parameter_aliases_referent_variable() {
        let f = file(
            "model Housing {\n  groove_width = 1.2mm [+0.05mm -0]\n}\n\
             model Ring {\n  groove_width = Housing.groove_width\n  \
             again = (groove_width)\n}\n",
        );
        let ev = run(&f);
        assert!(ev.diagnostics.is_empty(), "{:?}", ev.diagnostics);
        let h = ev.get(&at("Housing", "groove_width")).unwrap();
        let r = ev.get(&at("Ring", "groove_width")).unwrap();
        assert!(h.var.is_some());
        assert_eq!(r.var, h.var);
        assert_eq!(r.value, h.value);
        assert_eq!(r.alias_of, Some(at("Housing", "groove_width")));
        let again = ev.get(&at("Ring", "again")).unwrap();
        assert_eq!(again.var, h.var);
        assert_eq!(again.alias_of, Some(at("Housing", "groove_width")));
    }

    // frob:tests max_accessor_parameter_stores_untoleranced_9_5mm
    #[test]
    fn max_accessor_parameter_stores_untoleranced_9_5mm() {
        let f = file(
            "model Screw {\n  head_dia = 8.5mm [+0 -0.3mm]\n  \
             cb = head_dia.max + 1mm\n  want = 9.5mm\n  \
             nb = head_dia + 1mm\n}\n",
        );
        let ev = run(&f);
        assert!(ev.diagnostics.is_empty(), "{:?}", ev.diagnostics);
        let cb = quant(&ev, "Screw", "cb");
        assert_eq!(cb, quant(&ev, "Screw", "want"));
        assert_eq!(cb.form.bounds().unwrap(), (m(95, 10_000), m(95, 10_000)));
        assert!(ev.get(&at("Screw", "cb")).unwrap().var.is_none());
        assert_eq!(quant(&ev, "Screw", "nb"), cb);
    }

    // frob:tests dimension_mismatch_and_cycles_are_reported_once
    #[test]
    fn dimension_mismatch_and_cycles_are_reported_once() {
        let f = file("model M {\n  a = 1mm + 1kg\n  x = y + 1mm\n  y = x + 1mm\n}\n");
        let ev = run(&f);
        let codes: Vec<_> = ev.diagnostics.iter().map(|d| d.code).collect();
        assert_eq!(codes, vec!["Q002"]);
        assert!(ev.get(&at("M", "a")).is_none());
    }
}
