//! Kind and dimension checking of expressions (quantities.md 2 and 4, language.md 5).
//!
//! A [`KindChecker`] types an [`Expr`] bottom-up as a [`Ty`] (a [`KindedDim`] plus the
//! bare-number flag) and reports Q001, Q002, Q003, Q008, Q009 and Q011 as
//! [`KindDiagnostic`]s. It never evaluates a value: magnitudes belong to the evaluator.
//! Names are typed through a [`TypeEnv`] so the checker does not depend on resolution; an
//! unresolved name types as unknown and reports nothing (resolution reports E002), and an
//! unknown type never cascades into further diagnostics.
//!
//! Decisions made here (written into quantities.md 2 and 4.1):
//! - an untyped member assigned a bare number is an anonymous dimensionless value, not Q001;
//! - a bare number used with a dimensioned `+ - min max` operand or comparison is Q001;
//! - `x ^ n` with a dimensioned base needs an integer literal `n`, else Q002;
//! - `sin cos tan` take Angle: a bare argument is Q001, a wrong dimension Q002;
//! - an unknown kind, type or unit name is E002 (unresolved-name).

// frob:ticket 01M4SPWPKGXD3TJ78NPJH8Q9VZ

use std::collections::BTreeMap;

use cadetta_syntax::ast::{
    BinOp, Block, Component, Expr, ExprKind, Ident, Let, MemberKind, Path, PathSep, Type, TypeKind,
    UnOp,
};
use cadetta_syntax::span::Span;
use cadetta_units::{
    Dim, KindError, KindTable, KindWarning, KindedDim, Rational, UnitKind, UnitTable,
};
use tracing::{debug, trace};

use crate::deps::Severity;
use crate::eval::{Evaluation, Val};
use crate::resolve::{Ctx, Resolved, Resolver};

/// Accessors that return an untoleranced quantity of the receiver's type (quantities.md 5.2).
const ACCESSORS: &[&str] = &["nominal", "min", "max", "mid", "mmc", "lmc"];

/// A kind or dimension finding.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct KindDiagnostic {
    /// The lint code (`Q001`, `Q002`, `Q003`, `Q008`, `Q009`, `Q011`, `E002`).
    pub code: &'static str,
    /// Error, or warning for Q008.
    pub severity: Severity,
    /// The human message.
    pub message: String,
    /// The primary span.
    pub span: Span,
}

/// The type of an expression: kind, dimension and angle, and whether it is a bare number.
#[derive(Clone, PartialEq, Debug)]
pub struct Ty {
    /// Named or anonymous kind with dimension and angle exponent.
    pub kd: KindedDim,
    /// True for a bare number (anonymous dimensionless, quantities.md 2).
    pub bare: bool,
    /// The exact value of a bare number when it is known from literals alone.
    pub value: Option<Rational>,
}

impl Ty {
    fn quantity(kd: KindedDim) -> Self {
        Self {
            kd,
            bare: false,
            value: None,
        }
    }

    fn bare(value: Option<Rational>) -> Self {
        Self {
            kd: KindedDim::anonymous(Dim::default()),
            bare: true,
            value,
        }
    }

    fn is_scalar(&self) -> bool {
        scalar(&self.kd)
    }
}

fn scalar(kd: &KindedDim) -> bool {
    kd.dim.dimension.is_dimensionless() && kd.dim.angle == 0
}

fn int_value(v: &Rational) -> bool {
    *v == Rational::from_int(v.numer().clone())
}

/// Names the checker cannot type itself: members, parameters, constants.
pub trait TypeEnv {
    /// The type of a dotted or `::` path given as its segment names, when known.
    fn name_type(&self, segments: &[String]) -> Option<KindedDim>;

    /// Like [`name_type`](Self::name_type) but with the separator after each segment but the
    /// last, so a `::` qualified path can be told from a `.` member chain.
    fn path_type(&self, segments: &[String], _seps: &[PathSep]) -> Option<KindedDim> {
        self.name_type(segments)
    }

    /// The result type of a call to a user function, when known.
    fn call_type(&self, _name: &str, _args: &[Option<Ty>]) -> Option<Ty> {
        None
    }
}

/// A [`TypeEnv`] that knows nothing: every name types as unknown.
pub struct EmptyEnv;

impl TypeEnv for EmptyEnv {
    fn name_type(&self, _segments: &[String]) -> Option<KindedDim> {
        None
    }
}

/// A [`TypeEnv`] that types names through the resolver and the evaluated values, so a
/// cross-component read such as `Other.width` or `pkg::Mod::Comp.width` has its real
/// dimension (the evaluator owns magnitudes; only the dimension is taken here).
// frob:ticket 01M4TA3BXQVZSNE6JB2CH5HR2K
pub struct EvalEnv<'a> {
    resolver: &'a Resolver<'a>,
    evaluation: &'a Evaluation,
    ctx: Ctx,
}

impl<'a> EvalEnv<'a> {
    /// An environment for names written inside `component` of `module` in `package`.
    pub fn new(
        resolver: &'a Resolver<'a>,
        evaluation: &'a Evaluation,
        package: &str,
        module: &[String],
        component: &str,
    ) -> Self {
        Self {
            resolver,
            evaluation,
            ctx: Ctx {
                package: package.to_owned(),
                module: module.to_vec(),
                component: Some(component.to_owned()),
                locals: Vec::new(),
            },
        }
    }
}

impl TypeEnv for EvalEnv<'_> {
    fn name_type(&self, segments: &[String]) -> Option<KindedDim> {
        let seps = vec![PathSep::Dot; segments.len().saturating_sub(1)];
        self.path_type(segments, &seps)
    }

    fn path_type(&self, segments: &[String], seps: &[PathSep]) -> Option<KindedDim> {
        let span = Span::default();
        let path = Path {
            segments: segments
                .iter()
                .map(|name| Ident {
                    name: name.clone(),
                    span,
                })
                .collect(),
            seps: seps.to_vec(),
            span,
        };
        let Ok(Resolved::Member { path: member, .. }) =
            self.resolver.resolve_path(&self.ctx, &path, None)
        else {
            trace!(path = %path.text(), "kind env: not a resolvable member");
            return None;
        };
        let Val::Quantity(q) = &self.evaluation.get(&member)?.value else {
            return None;
        };
        trace!(path = %path.text(), "kind env: typed from evaluated member");
        Some(KindedDim::anonymous(q.dim().clone()))
    }
}

/// What an `as` or declared type names.
enum Target {
    Named(String),
    Anon(KindedDim),
}

/// Types expressions and collects Q-diagnostics.
pub struct KindChecker<'a> {
    kinds: &'a KindTable,
    units: &'a UnitTable,
    env: &'a dyn TypeEnv,
    locals: BTreeMap<String, Ty>,
    diags: Vec<KindDiagnostic>,
}

impl<'a> KindChecker<'a> {
    /// A checker over the given kind and unit tables and name environment.
    pub fn new(kinds: &'a KindTable, units: &'a UnitTable, env: &'a dyn TypeEnv) -> Self {
        Self {
            kinds,
            units,
            env,
            locals: BTreeMap::new(),
            diags: Vec::new(),
        }
    }

    /// The findings so far, in source order of discovery.
    pub fn diagnostics(&self) -> &[KindDiagnostic] {
        &self.diags
    }

    /// Consume the checker and return its findings.
    pub fn into_diagnostics(self) -> Vec<KindDiagnostic> {
        self.diags
    }

    fn report(&mut self, code: &'static str, severity: Severity, message: String, span: Span) {
        debug!(code, %message, "kind diagnostic");
        self.diags.push(KindDiagnostic {
            code,
            severity,
            message,
            span,
        });
    }

    fn kind_error(&mut self, e: &KindError, span: Span) {
        let code = e.code().unwrap_or("E002");
        self.report(code, Severity::Error, e.to_string(), span);
    }

    fn warnings(&mut self, ws: &[KindWarning], span: Span) {
        for w in ws {
            self.report(
                w.code(),
                Severity::Warning,
                format!("Q008 angle-dropped: {w:?}"),
                span,
            );
        }
    }

    fn q001(&mut self, span: Span, what: &str) {
        self.report(
            "Q001",
            Severity::Error,
            format!("Q001 unitless-quantity: a bare number cannot be {what}"),
            span,
        );
    }

    /// Check every parameter and `let` of `c` in dependency-insensitive fashion: members are
    /// typed in repeated silent passes until their types settle, then once more reporting.
    pub fn check_component(&mut self, c: &Component) {
        let saved = std::mem::take(&mut self.diags);
        for _ in 0..=c.members.len() {
            let before = self.locals.len();
            for m in &c.members {
                self.check_member(&m.kind);
            }
            self.diags.clear();
            if self.locals.len() == before {
                break;
            }
        }
        for m in &c.members {
            self.check_member(&m.kind);
        }
        let mut found = std::mem::take(&mut self.diags);
        self.diags = saved;
        self.diags.append(&mut found);
    }

    fn check_member(&mut self, kind: &MemberKind) {
        match kind {
            MemberKind::Param(p) => {
                let t = self.check_param(p.ty.as_ref(), &p.value);
                if let Some(t) = t {
                    self.locals.insert(p.name.name.clone(), t);
                }
            }
            MemberKind::Let(l) => self.check_let(l),
            _ => {}
        }
    }

    fn check_let(&mut self, l: &Let) {
        if let Some(t) = self.check_param(l.ty.as_ref(), &l.value) {
            self.locals.insert(l.name.name.clone(), t);
        }
    }

    /// Check `value` against an optional declared type and return the stored type. A bare
    /// number with no declared type is an anonymous dimensionless value.
    pub fn check_param(&mut self, ty: Option<&Type>, value: &Expr) -> Option<Ty> {
        let t = self.expr(value);
        let Some(ty) = ty else {
            return t.map(|t| Ty::quantity(t.kd));
        };
        let target = self.target(ty)?;
        let t = t?;
        Some(self.assign(&t, &target, value.span))
    }

    /// Check `value` assigned to the named kind `kind` (quantities.md 4.1 rule 4).
    pub fn check_assign(&mut self, value: &Expr, kind: &str) -> Option<Ty> {
        let t = self.expr(value)?;
        Some(self.assign(&t, &Target::Named(kind.to_string()), value.span))
    }

    fn target(&mut self, ty: &Type) -> Option<Target> {
        match &ty.kind {
            TypeKind::Path(p) => {
                let name = p.segments.last()?.name.clone();
                if self.kinds.dim_of(&name).is_none() {
                    // Interfaces, enums and components are not kinds; resolution owns them.
                    trace!(name, "declared type is not a kind; not checked");
                    return None;
                }
                Some(Target::Named(name))
            }
            TypeKind::KindExpr(ke) => {
                let mut text = String::new();
                for (i, t) in ke.terms.iter().enumerate() {
                    if i > 0 {
                        text.push(match t.op {
                            cadetta_syntax::ast::KindOp::Mul => '*',
                            cadetta_syntax::ast::KindOp::Div => '/',
                        });
                    }
                    text.push_str(&t.name.segments.last()?.name);
                    if let Some(e) = t.exp {
                        text.push_str(&format!("^{e}"));
                    }
                }
                match self.kinds.eval_expr(&text) {
                    Ok(kd) => Some(match kd.name() {
                        Some(n) => Target::Named(n.to_string()),
                        None => Target::Anon(kd),
                    }),
                    Err(e) => {
                        self.kind_error(&e, ty.span);
                        None
                    }
                }
            }
            _ => None,
        }
    }

    /// Assign `t` to `target`, reporting Q001/Q002/Q003/Q008/Q009; returns the stored type.
    fn assign(&mut self, t: &Ty, target: &Target, span: Span) -> Ty {
        match target {
            Target::Named(name) => {
                let Ok(named) = self.kinds.named(name) else {
                    return t.clone();
                };
                if t.bare {
                    if !scalar(&named) {
                        self.q001(span, &format!("assigned to a {name}"));
                    } else if name == "Count"
                        && let Some(v) = &t.value
                        && let Err(e) = self.kinds.count(v)
                    {
                        self.kind_error(&e, span);
                    }
                    return Ty::quantity(named);
                }
                match self.kinds.assign(&t.kd, name) {
                    Ok(a) => {
                        self.warnings(&a.warnings, span);
                        Ty::quantity(a.value)
                    }
                    Err(e) => {
                        self.kind_error(&e, span);
                        Ty::quantity(named)
                    }
                }
            }
            Target::Anon(kd) => {
                if t.bare {
                    if !scalar(kd) {
                        self.q001(span, "assigned to a dimensioned value");
                    }
                } else if t.kd.dim.check_same_dimension(&kd.dim).is_err() {
                    self.dim_mismatch(&t.kd, kd, span);
                } else if t.kd.dim.angle != kd.dim.angle {
                    self.warnings(
                        &[KindWarning::AngleDropped {
                            found: t.kd.dim.angle,
                            expected: kd.dim.angle,
                        }],
                        span,
                    );
                }
                Ty::quantity(kd.clone())
            }
        }
    }

    fn dim_mismatch(&mut self, a: &KindedDim, b: &KindedDim, span: Span) {
        let e = KindError::DimensionMismatch {
            left: a.dim.dimension.render(),
            right: b.dim.dimension.render(),
        };
        self.kind_error(&e, span);
    }

    /// The type of an expression, or `None` when it is unknown or not a quantity.
    pub fn expr(&mut self, e: &Expr) -> Option<Ty> {
        let out = self.expr_inner(e);
        trace!(span = ?e.span, ty = ?out.as_ref().map(|t| t.kd.name().map(str::to_string)), "typed");
        out
    }

    fn expr_inner(&mut self, e: &Expr) -> Option<Ty> {
        match &e.kind {
            ExprKind::Number(text) => Some(Ty::bare(Rational::parse_decimal(text).ok())),
            ExprKind::Quantity(segs) => {
                let unit = &segs.first()?.unit;
                match self.units.parse(unit) {
                    Ok(r) => Some(Ty::quantity(self.kinds.from_unit(&r))),
                    Err(err) => {
                        self.report(
                            "E002",
                            Severity::Error,
                            format!("E002 unresolved-name: unit `{unit}`: {err}"),
                            e.span,
                        );
                        None
                    }
                }
            }
            ExprKind::Str(_) | ExprKind::Bool(_) => None,
            ExprKind::Path(_) | ExprKind::Field(..) | ExprKind::Index(..) => self.reference(e),
            ExprKind::Paren(inner) => self.expr(inner),
            ExprKind::Array(items) => {
                let mut acc: Option<Ty> = None;
                for it in items {
                    let t = self.expr(it);
                    acc = match (acc, t) {
                        (Some(a), Some(b)) => self.operands(&a, &b, it.span, it.span, it.span),
                        (None, t) => t,
                        (a, None) => a,
                    };
                }
                None
            }
            ExprKind::Unary(UnOp::Neg, inner) => {
                let mut t = self.expr(inner)?;
                t.value = t.value.map(|v| v.neg());
                Some(t)
            }
            ExprKind::Unary(UnOp::Not, inner) => {
                self.expr(inner);
                None
            }
            ExprKind::Binary(op, l, r) => self.binary(*op, l, r, e.span),
            ExprKind::Compare { first, rest } => {
                let mut prev = (self.expr(first), first.span);
                for (op, next) in rest {
                    let cur = self.expr(next);
                    if let (Some(a), Some(b)) = (&prev.0, &cur) {
                        trace!(?op, "comparison operands");
                        self.operands(a, b, prev.1, next.span, e.span);
                    }
                    prev = (cur, next.span);
                }
                None
            }
            ExprKind::Satisfies(l, r) => {
                self.expr(l);
                self.expr(r);
                None
            }
            ExprKind::As(inner, ty) => {
                let t = self.expr(inner);
                let target = self.target(ty)?;
                self.cast(t?, target, e.span)
            }
            ExprKind::Call { callee, args, .. } => self.call(callee, args, e.span),
            ExprKind::Tolerance(inner, terms) => {
                let t = self.expr(inner)?;
                for term in terms {
                    if let Some(d) = self.expr(&term.value) {
                        self.tolerance_term(&t, &d, term.value.span);
                    }
                }
                Some(t)
            }
            ExprKind::IsoClass(inner, _) => self.expr(inner),
            ExprKind::If {
                cond,
                then,
                otherwise,
            } => {
                self.expr(cond);
                let a = self.block(then);
                let b = otherwise.as_ref().and_then(|o| self.expr(o));
                match (a, b) {
                    (Some(a), Some(b)) => self.operands(&a, &b, e.span, e.span, e.span),
                    (a, None) => a,
                    (None, b) => b,
                }
            }
            ExprKind::Match { scrutinee, arms } => {
                self.expr(scrutinee);
                let mut acc: Option<Ty> = None;
                for arm in arms {
                    let t = self.expr(&arm.body);
                    acc = match (acc, t) {
                        (Some(a), Some(b)) => {
                            self.operands(&a, &b, arm.body.span, arm.body.span, arm.body.span)
                        }
                        (None, t) => t,
                        (a, None) => a,
                    };
                }
                acc
            }
            ExprKind::Block(b) => self.block(b),
        }
    }

    fn block(&mut self, b: &Block) -> Option<Ty> {
        let saved = self.locals.clone();
        for l in &b.lets {
            self.check_let(l);
        }
        let out = self.expr(&b.tail);
        self.locals = saved;
        out
    }

    /// Type a name, field chain or index: locals first, then the environment; a trailing
    /// accessor (`.max`) has the type of its receiver.
    fn reference(&mut self, e: &Expr) -> Option<Ty> {
        if let ExprKind::Index(base, idx) = &e.kind {
            if let Some(i) = self.expr(idx)
                && !i.bare
                && !matches!(i.kd.name(), Some("Count"))
                && !i.is_scalar()
            {
                self.dim_mismatch(&i.kd, &KindedDim::anonymous(Dim::default()), idx.span);
            }
            return self.expr(base);
        }
        let mut segs = Vec::new();
        let mut seps = Vec::new();
        if !flatten(e, &mut segs, &mut seps) {
            if let ExprKind::Field(inner, _) = &e.kind {
                return self.expr(inner);
            }
            return None;
        }
        self.lookup(&segs, &seps)
            .or_else(|| match segs.split_last() {
                Some((last, rest)) if ACCESSORS.contains(&last.as_str()) && !rest.is_empty() => {
                    self.lookup(rest, &seps[..rest.len() - 1])
                }
                _ => None,
            })
    }

    fn lookup(&self, segs: &[String], seps: &[PathSep]) -> Option<Ty> {
        if let [one] = segs
            && let Some(t) = self.locals.get(one)
        {
            return Some(t.clone());
        }
        self.env.path_type(segs, seps).map(Ty::quantity)
    }

    fn binary(&mut self, op: BinOp, l: &Expr, r: &Expr, span: Span) -> Option<Ty> {
        let a = self.expr(l);
        let b = self.expr(r);
        match op {
            BinOp::And | BinOp::Or => None,
            BinOp::Add | BinOp::Sub => {
                let (a, b) = (a?, b?);
                if a.bare && b.bare {
                    let value = match (&a.value, &b.value) {
                        (Some(x), Some(y)) if op == BinOp::Add => Some(x.add(y)),
                        (Some(x), Some(y)) => Some(x.sub(y)),
                        _ => None,
                    };
                    return Some(Ty::bare(value));
                }
                self.operands(&a, &b, l.span, r.span, span)
            }
            BinOp::Mul | BinOp::Div => {
                let (a, b) = (a?, b?);
                if a.bare && b.bare {
                    let value = match (&a.value, &b.value) {
                        (Some(x), Some(y)) if op == BinOp::Mul => Some(x.mul(y)),
                        (Some(x), Some(y)) => x.div(y).ok(),
                        _ => None,
                    };
                    return Some(Ty::bare(value));
                }
                let (ka, kb) = (self.factor(&a), self.factor(&b));
                let kd = if op == BinOp::Mul {
                    self.kinds.mul(&ka, &kb)
                } else {
                    self.kinds.div(&ka, &kb)
                };
                Some(Ty::quantity(kd))
            }
            BinOp::Pow => self.pow(a?, r, span),
            BinOp::Range => {
                let (a, b) = (a?, b?);
                self.operands(&a, &b, l.span, r.span, span)
            }
            BinOp::In => {
                // `x in lo..hi`: x is compared against the interval's type.
                if let (Some(a), Some(b)) = (a, b) {
                    self.operands(&a, &b, l.span, r.span, span);
                }
                None
            }
        }
    }

    /// A bare integer factor acts as a Count so it scales without changing a Count to a Ratio.
    fn factor(&self, t: &Ty) -> KindedDim {
        match (&t.value, t.bare) {
            (Some(v), true) if int_value(v) => {
                self.kinds.named("Count").unwrap_or_else(|_| t.kd.clone())
            }
            _ => t.kd.clone(),
        }
    }

    fn pow(&mut self, base: Ty, exp: &Expr, span: Span) -> Option<Ty> {
        if base.bare {
            return Some(Ty::bare(None));
        }
        let n = int_literal(exp);
        match n {
            Some(n) => Some(Ty::quantity(self.kinds.powi(&base.kd, n))),
            None if base.is_scalar() => Some(Ty::quantity(KindedDim::anonymous(Dim::default()))),
            None => {
                self.report(
                    "Q002",
                    Severity::Error,
                    "Q002 dimension-mismatch: the exponent of a dimensioned base must be an \
                     integer literal"
                        .to_string(),
                    span,
                );
                None
            }
        }
    }

    /// Combine two operands of `+ - min max`, comparison or a range (quantities.md 4.1 rule 2,
    /// bare numbers per quantities.md 2). Returns the result type, or `None` after a Q002/Q003.
    fn operands(&mut self, a: &Ty, b: &Ty, sa: Span, sb: Span, span: Span) -> Option<Ty> {
        match (a.bare, b.bare) {
            (true, true) => Some(Ty::bare(None)),
            (true, false) | (false, true) => {
                let (bare_span, other) = if a.bare { (sa, b) } else { (sb, a) };
                if !other.is_scalar() {
                    self.q001(bare_span, "an operand beside a dimensioned value");
                }
                Some(Ty::quantity(other.kd.clone()))
            }
            (false, false) => match self.kinds.combine(&a.kd, &b.kd) {
                Ok(c) => {
                    self.warnings(&c.warnings, span);
                    Some(Ty::quantity(c.value))
                }
                Err(e) => {
                    self.kind_error(&e, span);
                    None
                }
            },
        }
    }

    fn tolerance_term(&mut self, nominal: &Ty, dev: &Ty, span: Span) {
        if dev.bare {
            // `[+0 -0.008mm]`: a bare zero needs no unit; any other bare deviation of a
            // dimensioned nominal is Q001.
            let zero = dev.value.as_ref().is_some_and(Rational::is_zero);
            if !zero && !nominal.is_scalar() {
                self.q001(span, "a tolerance deviation of a dimensioned value");
            }
        } else if nominal.kd.dim.check_same_dimension(&dev.kd.dim).is_err() {
            self.dim_mismatch(&nominal.kd, &dev.kd, span);
        }
    }

    fn cast(&mut self, t: Ty, target: Target, span: Span) -> Option<Ty> {
        let kd = match &target {
            Target::Named(n) => match self.kinds.named(n) {
                Ok(kd) => kd,
                Err(e) => {
                    self.kind_error(&e, span);
                    return None;
                }
            },
            Target::Anon(kd) => kd.clone(),
        };
        if t.bare {
            if !scalar(&kd) {
                self.q001(span, "converted with `as` to a dimensioned kind");
            }
        } else if t.kd.dim.check_same_dimension(&kd.dim).is_err() {
            self.dim_mismatch(&t.kd, &kd, span);
        }
        Some(Ty::quantity(kd))
    }

    fn call(&mut self, callee: &Expr, args: &[cadetta_syntax::ast::Arg], span: Span) -> Option<Ty> {
        let tys: Vec<Option<Ty>> = args.iter().map(|a| self.expr(&a.value)).collect();
        let ExprKind::Path(p) = &callee.kind else {
            return None;
        };
        let [seg] = p.segments.as_slice() else {
            return None;
        };
        let name = seg.name.as_str();
        let arg_span = |i: usize| args.get(i).map_or(span, |a| a.value.span);
        let fold = |this: &mut Self| -> Option<Ty> {
            let mut acc: Option<Ty> = None;
            for (i, t) in tys.iter().enumerate() {
                let Some(t) = t else { return None };
                acc = match acc {
                    None => Some(t.clone()),
                    Some(a) => this.operands(&a, t, arg_span(0), arg_span(i), span),
                };
            }
            acc
        };
        match name {
            "min" | "max" => fold(self),
            "abs" => tys.first().cloned().flatten(),
            "round_to" | "ceil_to" | "floor_to" => fold(self),
            "sqrt" | "cbrt" => {
                let t = tys.first().cloned().flatten()?;
                if t.bare {
                    return Some(Ty::quantity(KindedDim::anonymous(Dim::default())));
                }
                let index = if name == "sqrt" { 2 } else { 3 };
                match self.kinds.root(&t.kd, index) {
                    Ok(kd) => Some(Ty::quantity(kd)),
                    Err(e) => {
                        self.kind_error(&e, span);
                        None
                    }
                }
            }
            "sin" | "cos" | "tan" => {
                let t = tys.first().cloned().flatten()?;
                if t.bare {
                    self.q001(
                        arg_span(0),
                        &format!("the argument of {name}; give an Angle"),
                    );
                } else if !t.kd.dim.dimension.is_dimensionless() {
                    let angle = self.kinds.named("Angle").ok()?;
                    self.dim_mismatch(&t.kd, &angle, arg_span(0));
                } else if t.kd.dim.angle != 1 {
                    self.warnings(
                        &[KindWarning::AngleDropped {
                            found: t.kd.dim.angle,
                            expected: 1,
                        }],
                        arg_span(0),
                    );
                }
                self.kinds.named("Ratio").ok().map(Ty::quantity)
            }
            "atan2" => {
                fold(self);
                self.kinds.named("Angle").ok().map(Ty::quantity)
            }
            "floor" | "ceil" | "round" => {
                if let Some(t) = tys.first().cloned().flatten()
                    && !t.bare
                    && !t.is_scalar()
                {
                    let none = KindedDim::anonymous(Dim::default());
                    self.dim_mismatch(&t.kd, &none, arg_span(0));
                }
                self.kinds.named("Count").ok().map(Ty::quantity)
            }
            "count" => self.kinds.named("Count").ok().map(Ty::quantity),
            _ => self.env.call_type(name, &tys),
        }
    }
}

/// Flatten a `Path` and `.field` chain into segment names and their separators; false when
/// it is anything else.
fn flatten(e: &Expr, out: &mut Vec<String>, seps: &mut Vec<PathSep>) -> bool {
    match &e.kind {
        ExprKind::Path(p) => {
            out.extend(p.segments.iter().map(|s| s.name.clone()));
            seps.extend(p.seps.iter().copied());
            true
        }
        ExprKind::Field(inner, f) => {
            let ok = flatten(inner, out, seps);
            out.push(f.name.clone());
            seps.push(PathSep::Dot);
            ok
        }
        ExprKind::Index(inner, _) => flatten(inner, out, seps),
        ExprKind::Paren(inner) => flatten(inner, out, seps),
        _ => false,
    }
}

/// The value of an integer literal exponent (`2`, `-3`, `(2)`), when it is one.
fn int_literal(e: &Expr) -> Option<i32> {
    match &e.kind {
        ExprKind::Number(t) => t.parse().ok(),
        ExprKind::Paren(i) => int_literal(i),
        ExprKind::Unary(UnOp::Neg, i) => int_literal(i).map(|n| -n),
        _ => None,
    }
}

/// True when a unit kind names a kind (used by callers deciding whether to look it up).
pub fn is_named(k: &UnitKind) -> bool {
    matches!(k, UnitKind::Named(_))
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadetta_syntax::parser::parse;

    struct Env(KindTable);

    impl TypeEnv for Env {
        fn name_type(&self, s: &[String]) -> Option<KindedDim> {
            let kind = match s.join(".").as_str() {
                "some_torque" => "Torque",
                "len" => "Length",
                "eta" => "Ratio",
                "lead" => "Lead",
                "rpm" => "AngularVelocity",
                "n" => "Count",
                "area" => "Area",
                "angle" => "Angle",
                _ => return None,
            };
            self.0.named(kind).ok()
        }
    }

    /// Check a model `M` whose members are `src`; return codes in order.
    fn codes(src: &str) -> Vec<&'static str> {
        let p = parse(&format!("model M {{\n{src}\n}}\n"));
        assert!(p.diags.is_empty(), "{src}: {:?}", p.diags);
        let kinds = KindTable::builtin();
        let units = UnitTable::builtin();
        let env = Env(kinds.clone());
        let mut c = KindChecker::new(&kinds, &units, &env);
        for comp in p.file.components() {
            c.check_component(comp);
        }
        c.into_diagnostics().iter().map(|d| d.code).collect()
    }

    // frob:tests crates/cadetta-sema/src/kindcheck.rs::KindChecker
    #[test]
    fn bare_number_assigned_to_length_is_q001() {
        assert_eq!(codes("a: Length = 5"), ["Q001"]);
        assert!(codes("a: Length = 5mm").is_empty());
    }

    // frob:tests crates/cadetta-sema/src/kindcheck.rs::KindChecker
    #[test]
    fn adding_length_and_mass_is_q002() {
        assert_eq!(codes("a = 1mm + 1kg"), ["Q002"]);
        assert!(codes("a = 1mm + 2in").is_empty());
    }

    // frob:tests crates/cadetta-sema/src/kindcheck.rs::KindChecker
    #[test]
    fn named_kind_needs_as_conversion_q003() {
        // Torque (angle -1) to Energy: same dimension, different named kind.
        assert_eq!(codes("x: Energy = some_torque"), ["Q003"]);
        assert!(codes("x: Energy = some_torque as Energy").is_empty());
    }

    // frob:tests crates/cadetta-sema/src/kindcheck.rs::KindChecker
    #[test]
    fn angle_cases_of_4_3_fire_exactly_on_stated_rows() {
        assert!(codes("v: Velocity = 1500rpm * 8mm/rev").is_empty());
        assert_eq!(codes("v: Velocity = 1500rpm * 8mm"), ["Q008"]);
        assert!(codes("f: Force = 0.45N*m * eta / 8mm/rev").is_empty());
        assert_eq!(codes("f: Force = 0.45N*m * eta / 8mm"), ["Q008"]);
        assert!(codes("t: Torque = 50N * 9mm").is_empty());
    }

    // frob:tests crates/cadetta-sema/src/kindcheck.rs::KindChecker
    #[test]
    fn non_integer_count_and_bad_root_are_q009_q011() {
        assert_eq!(codes("c: Count = 2.5"), ["Q009"]);
        assert!(codes("c: Count = 3").is_empty());
        assert_eq!(codes("a = sqrt(1mm^3)"), ["Q011"]);
        assert!(codes("a: Length = sqrt(area)").is_empty());
        assert_eq!(codes("a = cbrt(1mm^2)"), ["Q011"]);
    }

    // frob:tests crates/cadetta-sema/src/kindcheck.rs::KindChecker
    #[test]
    fn legal_bare_numbers_are_not_q001() {
        for src in [
            "a: Length = 2 * 3mm",
            "a: Length = 3mm / 2",
            "a: Length = len / 4",
            "a: Area = len ^ 2",
            "r: Ratio = 0.5",
            "c: Count = 4",
            "c: Count = n + 1",
            "r: Ratio = eta + 0.1",
            "a = 1 + 2",
            "c: Count = floor(len / 3mm) + 1",
            "a = 10mm [+0 -0.008mm]",
            "a: Length = 20mm H7",
        ] {
            assert!(codes(src).is_empty(), "{src}: {:?}", codes(src));
        }
        assert_eq!(codes("a: Length = len + 2"), ["Q001"]);
    }

    // frob:tests crates/cadetta-sema/src/kindcheck.rs::KindChecker
    #[test]
    fn members_resolve_in_any_order_and_unknowns_do_not_cascade() {
        assert_eq!(codes("b: Mass = a\na = 1mm"), ["Q002"]);
        assert!(codes("a: Length = missing + 1mm").is_empty());
        assert!(codes("a: Length = len.max + 1mm").is_empty());
    }
}
