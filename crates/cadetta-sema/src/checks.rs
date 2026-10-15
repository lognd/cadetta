//! `check` and `require` statements with three-valued verdicts
//! (language.md 6, quantities.md 5.7, 5.8).
//!
//! A check condition is a comparison chain, `x satisfies P`, or `and`, `or`
//! and `not` over those, combined with Kleene logic over `holds`, `violates`
//! and `undecided`. Each check yields one [`CheckResult`] (one per instance
//! for `for x in ...`), and C001 (violates, error), C002 (undecided, warning)
//! and C003 (`==` on toleranced values, warning) diagnostics.
//!
//! Decisions made here (written into language.md 6): a bound is looked up
//! syntactically, either bare (`P`, in the enclosing component) or as
//! `Item.P`, because bounds are not value members; when a parameter and a
//! bound share a name, `P.min`/`P.max` read the parameter and `satisfies P`
//! reads the bound. A bound's limits are the nominals of its expressions.
//! A quantifier ranges over an array literal; ranging over a `part` array
//! waits for the parts ticket. A `require` carries its own citation, else its
//! component's, and `obligation` marks it as tracked (obligations.md).

// frob:ticket 01M4SPWPT5XR3QCQJ7M1XP07MP

use std::collections::BTreeMap;
use std::rc::Rc;

use cadetta_syntax::ast::{
    self, BinOp, BoundKind, CmpOp as AstCmp, Component, Expr, ExprKind, MemberKind, UnOp,
};
use cadetta_syntax::span::Span;
use cadetta_units::compare::{CmpOp, Note, Operand, Verdict, compare};
use cadetta_units::{Bound, InexactInterval};
use tracing::{debug, trace, warn};

use super::instances::InstInfo;
use super::{Evaluator, Fail, Quant, R, Scope, Val, diag, strip, tol_diag};
use crate::cite::effective;
use crate::deps::Severity;
use crate::eval::EvalDiagnostic;
use crate::resolve::{CanonPath, Ctx, MemberSeg, ModuleKey, Resolved};

/// The verdict of one `check` or `require` (for one instance when quantified).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct CheckResult {
    /// The check's canonical path (`PKG::Item.name`).
    pub path: CanonPath,
    /// The position within the quantifier, absent for an unquantified check.
    pub instance: Option<usize>,
    /// Holds, violates or undecided.
    pub verdict: Verdict,
    /// Caveats on the verdict.
    pub notes: Vec<Note>,
    /// The optional message string.
    pub message: Option<String>,
    /// The cited external requirement, for a `require`.
    pub citation: Option<ast::Citation>,
    /// True for a `require`, which is tracked as an obligation.
    pub obligation: bool,
    /// The statement's span.
    pub span: Span,
}

/// A check or require, indexed for evaluation.
#[derive(Clone)]
struct CheckDef<'p> {
    path: CanonPath,
    each: Option<&'p (ast::Ident, Expr)>,
    cond: &'p Expr,
    message: Option<String>,
    citation: Option<ast::Citation>,
    obligation: bool,
    ctx: Ctx,
    span: Span,
    /// The instance whose members the condition reads (`None` for a plain component).
    inst: Option<Rc<InstInfo>>,
    /// The `#[when(cond)]` gate: the check exists only while it holds.
    when: Option<&'p Expr>,
}

/// A bound member, indexed by its component and name.
#[derive(Clone)]
struct BoundDef<'p> {
    kind: &'p BoundKind,
    ctx: Ctx,
}

/// The checks and bounds of a project, in source order.
#[derive(Default)]
pub struct CheckIndex<'p> {
    checks: Vec<CheckDef<'p>>,
    bounds: BTreeMap<CanonPath, BoundDef<'p>>,
}

impl<'p> CheckIndex<'p> {
    /// Indexes the checks, requires and bounds of a component without parameters.
    pub fn add_component(&mut self, package: &str, module: &[String], c: &'p Component) {
        let parameterized = !c.params.is_empty();
        if parameterized {
            trace!(component = %c.name.name, "checks: parameterized component, checks run per instance");
        }
        let ctx = Ctx {
            package: package.to_owned(),
            module: module.to_vec(),
            component: Some(c.name.name.clone()),
            locals: Vec::new(),
        };
        let item = CanonPath::item(package, module, &c.name.name);
        let at = |name: &str| {
            let mut p = item.clone();
            p.members.push(MemberSeg::Name(name.to_owned()));
            p
        };
        for m in &c.members {
            match &m.kind {
                MemberKind::Check(_) | MemberKind::Require(_) if parameterized => {}
                MemberKind::Check(k) => self.checks.push(CheckDef {
                    path: at(&k.name.name),
                    each: k.each.as_ref(),
                    cond: &k.cond,
                    message: k.message.clone(),
                    citation: None,
                    obligation: false,
                    ctx: ctx.clone(),
                    span: m.span,
                    inst: None,
                    when: None,
                }),
                MemberKind::Require(r) => self.checks.push(CheckDef {
                    path: at(&r.name.name),
                    each: None,
                    cond: &r.cond,
                    message: r.message.clone(),
                    citation: effective(r.citation.as_ref(), c.citation.as_ref()).cloned(),
                    obligation: true,
                    ctx: ctx.clone(),
                    span: m.span,
                    inst: None,
                    when: None,
                }),
                MemberKind::Bound(b) => {
                    let name = match &b.kind {
                        BoundKind::Le { name, .. }
                        | BoundKind::Ge { name, .. }
                        | BoundKind::In { name, .. } => &name.name,
                        BoundKind::Eq { .. } => continue,
                    };
                    self.bounds.insert(
                        at(name),
                        BoundDef {
                            kind: &b.kind,
                            ctx: ctx.clone(),
                        },
                    );
                }
                _ => {}
            }
        }
    }
}

/// The result of deciding one condition.
struct Outcome {
    verdict: Verdict,
    notes: Vec<Note>,
}

impl Outcome {
    fn of(verdict: Verdict) -> Self {
        Self {
            verdict,
            notes: Vec::new(),
        }
    }
}

fn not(v: Verdict) -> Verdict {
    match v {
        Verdict::Holds => Verdict::Violates,
        Verdict::Violates => Verdict::Holds,
        Verdict::Undecided => Verdict::Undecided,
    }
}

/// Kleene conjunction: violates beats undecided beats holds.
fn and(a: Verdict, b: Verdict) -> Verdict {
    match (a, b) {
        (Verdict::Violates, _) | (_, Verdict::Violates) => Verdict::Violates,
        (Verdict::Undecided, _) | (_, Verdict::Undecided) => Verdict::Undecided,
        _ => Verdict::Holds,
    }
}

fn or(a: Verdict, b: Verdict) -> Verdict {
    not(and(not(a), not(b)))
}

fn join(a: Outcome, b: Outcome, f: fn(Verdict, Verdict) -> Verdict) -> Outcome {
    let mut notes = a.notes;
    for n in b.notes {
        if !notes.contains(&n) {
            notes.push(n);
        }
    }
    Outcome {
        verdict: f(a.verdict, b.verdict),
        notes,
    }
}

/// True when `callee` is the bare name `name`.
fn is_named(callee: &Expr, name: &str) -> bool {
    matches!(&strip(callee).kind, ExprKind::Path(p) if p.segments.len() == 1 && p.segments[0].name == name)
}

fn units_op(op: AstCmp) -> CmpOp {
    match op {
        AstCmp::Eq => CmpOp::Eq,
        AstCmp::Ne => CmpOp::Ne,
        AstCmp::Lt => CmpOp::Lt,
        AstCmp::Le => CmpOp::Le,
        AstCmp::Gt => CmpOp::Gt,
        AstCmp::Ge => CmpOp::Ge,
    }
}

impl<'r, 'p> Evaluator<'r, 'p> {
    /// The checks and requires of every instance, keyed under the instance prefix.
    // frob:ticket 01M4W5HM6SAY12P1J3R3G80Q7F
    fn instance_check_defs(&self) -> Vec<CheckDef<'p>> {
        let mut out = Vec::new();
        for (info, comp, ctx, _) in self.instances.members() {
            let at = |name: &str| {
                let mut p = info.prefix.clone();
                p.members.push(MemberSeg::Name(name.to_owned()));
                p
            };
            for m in &comp.members {
                let when = m
                    .attrs
                    .iter()
                    .chain(m.trailing.iter())
                    .find(|a| a.name.name == "when")
                    .and_then(|a| match a.args.first() {
                        Some(ast::AttrArg::Expr(e)) => Some(e),
                        _ => None,
                    });
                let def = match &m.kind {
                    MemberKind::Check(k) => CheckDef {
                        path: at(&k.name.name),
                        each: k.each.as_ref(),
                        cond: &k.cond,
                        message: k.message.clone(),
                        citation: None,
                        obligation: false,
                        ctx: ctx.clone(),
                        span: m.span,
                        inst: Some(info.clone()),
                        when,
                    },
                    MemberKind::Require(r) => CheckDef {
                        path: at(&r.name.name),
                        each: None,
                        cond: &r.cond,
                        message: r.message.clone(),
                        citation: effective(r.citation.as_ref(), comp.citation.as_ref()).cloned(),
                        obligation: true,
                        ctx: ctx.clone(),
                        span: m.span,
                        inst: Some(info.clone()),
                        when,
                    },
                    _ => continue,
                };
                out.push(def);
            }
        }
        debug!(checks = out.len(), "checks: instance checks indexed");
        out
    }

    /// Evaluates every indexed check and require, recording C001, C002 and C003.
    pub(super) fn run_checks(&mut self) -> Vec<CheckResult> {
        let mut defs = self.index.checks.clone();
        defs.extend(self.instance_check_defs());
        let mut out = Vec::new();
        for def in &defs {
            let mut sc = Scope {
                ctx: def.ctx.clone(),
                locals: Vec::new(),
            };
            let start = self.diagnostics.len();
            let saved = std::mem::replace(&mut self.cur_inst, def.inst.clone());
            let outcome = match self.gate(&mut sc, def) {
                Ok(true) => self.run_check(&mut sc, def),
                Ok(false) => {
                    debug!(check = %def.path, "checks: `when` is false, check absent");
                    Ok(Vec::new())
                }
                Err(f) => Err(f),
            };
            self.cur_inst = saved;
            // C003 and other pushes made while the check ran carry the def's module.
            self.stamp_since(start, &ModuleKey::of_ctx(&def.ctx));
            match outcome {
                Ok(rs) => out.extend(rs),
                Err(Fail::Silent) => {}
                Err(Fail::Diag(mut d)) => {
                    if let Some(i) = &def.inst {
                        // The callee's source is std or another package: report at the `part` site.
                        debug!(check = %def.path, code = d.code, "checks: instance failure reported at the part");
                        d.span = i.span;
                        d.module = Some(ModuleKey::of_path(&i.prefix));
                    }
                    self.diagnostics
                        .push(d.in_module(ModuleKey::of_ctx(&def.ctx)));
                }
            }
        }
        debug!(checks = out.len(), "checks: evaluation finished");
        out
    }

    /// True when the check's `#[when]` gate (if any) holds.
    fn gate(&mut self, sc: &mut Scope, def: &CheckDef<'p>) -> R<bool> {
        match def.when {
            None => Ok(true),
            Some(cond) => match self.eval(sc, cond)? {
                Val::Bool(b) => Ok(b),
                _ => Err(diag("EVAL", cond.span, "a `when` condition must be a Bool")),
            },
        }
    }

    fn run_check(&mut self, sc: &mut Scope, def: &CheckDef<'p>) -> R<Vec<CheckResult>> {
        let mut results = Vec::new();
        match def.each {
            None => {
                let o = self.condition(sc, def.cond)?;
                results.push(self.record(def, None, o));
            }
            Some((var, source)) => {
                let items = self.instances(sc, source)?;
                debug!(check = %def.path, n = items.len(), "checks: quantified over instances");
                for (i, v) in items.into_iter().enumerate() {
                    sc.push(&var.name, v);
                    let o = self.condition(sc, def.cond);
                    sc.truncate(0);
                    results.push(self.record(def, Some(i), o?));
                }
            }
        }
        Ok(results)
    }

    /// The values a `for x in source` quantifier ranges over.
    fn instances(&mut self, sc: &mut Scope, source: &Expr) -> R<Vec<Val>> {
        match &strip(source).kind {
            ExprKind::Array(items) => items.iter().map(|e| self.eval(sc, e)).collect(),
            ExprKind::Path(_) | ExprKind::Field(..) => {
                if let Ok(Resolved::Part { .. }) = self.resolver.resolve_ref(&sc.ctx, source, None)
                {
                    return Err(diag(
                        "EVAL",
                        source.span,
                        "ranging over part instances is evaluated by the parts ticket",
                    ));
                }
                Err(diag(
                    "EVAL",
                    source.span,
                    "a quantifier ranges over an array or a part array",
                ))
            }
            _ => Err(diag(
                "EVAL",
                source.span,
                "a quantifier ranges over an array or a part array",
            )),
        }
    }

    fn record(&mut self, def: &CheckDef<'p>, instance: Option<usize>, o: Outcome) -> CheckResult {
        let name = def.path.to_string();
        let which = instance.map_or(String::new(), |i| format!(" [instance {i}]"));
        let text = def
            .message
            .as_deref()
            .map_or(String::new(), |m| format!(": {m}"));
        let (code, severity, what) = match o.verdict {
            Verdict::Violates => ("C001", Severity::Error, "violates"),
            Verdict::Undecided => ("C002", Severity::Warning, "is undecided"),
            Verdict::Holds => {
                trace!(%name, ?instance, "checks: holds");
                ("", Severity::Warning, "holds")
            }
        };
        if !code.is_empty() {
            let kind = if def.obligation { "require" } else { "check" };
            let mut message = format!("{kind} `{name}`{which} {what}{text}");
            if o.notes.contains(&Note::Conservative) {
                message.push_str(" (conservative evaluation)");
            }
            if o.notes.contains(&Note::Inexact) {
                message.push_str(" (inexact operand within 1e-9 relative)");
            }
            warn!(code, %message, "checks: verdict");
            self.diagnostics.push(EvalDiagnostic {
                module: Some(match &def.inst {
                    Some(i) => ModuleKey::of_path(&i.prefix),
                    None => ModuleKey::of_ctx(&def.ctx),
                }),
                code,
                severity,
                message,
                span: def.span,
            });
        }
        CheckResult {
            path: def.path.clone(),
            instance,
            verdict: o.verdict,
            notes: o.notes,
            message: def.message.clone(),
            citation: def.citation.clone(),
            obligation: def.obligation,
            span: def.span,
        }
    }

    /// Decides a condition expression to a verdict.
    fn condition(&mut self, sc: &mut Scope, e: &Expr) -> R<Outcome> {
        match &e.kind {
            ExprKind::Paren(inner) => self.condition(sc, inner),
            ExprKind::Bool(b) => Ok(Outcome::of(if *b {
                Verdict::Holds
            } else {
                Verdict::Violates
            })),
            ExprKind::Unary(UnOp::Not, x) => {
                let mut o = self.condition(sc, x)?;
                o.verdict = not(o.verdict);
                Ok(o)
            }
            ExprKind::Binary(op @ (BinOp::And | BinOp::Or), l, r) => {
                let a = self.condition(sc, l)?;
                let b = self.condition(sc, r)?;
                Ok(join(a, b, if *op == BinOp::And { and } else { or }))
            }
            ExprKind::Compare { first, rest } => {
                let before = self.float_reads;
                let mut left = self.operand(sc, first)?;
                let mut acc: Option<Outcome> = None;
                for (op, rhs) in rest {
                    let right = self.operand(sc, rhs)?;
                    let o = self.compare_pair(units_op(*op), &left, &right, e.span)?;
                    acc = Some(match acc {
                        None => o,
                        Some(prev) => join(prev, o, and),
                    });
                    left = right;
                }
                let out =
                    acc.ok_or_else(|| diag("EVAL", e.span, "a comparison needs an operator"))?;
                self.note_float_read(before, out.verdict, e.span);
                Ok(out)
            }
            ExprKind::Satisfies(x, p) => self.satisfies(sc, x, p, e.span),
            // frob:ticket 01M4VS6WG57MAD0T8GK956NGG9
            ExprKind::Call { callee, args, .. } if is_named(callee, "aligned") => {
                let (verdict, notes) = self.aligned_condition(sc, args, e.span)?;
                Ok(Outcome { verdict, notes })
            }
            // frob:ticket 01M4V43BJ1Z4EZTYJXMMCJWA2D
            ExprKind::Call { callee, args, .. }
                if is_named(callee, "coaxial") || is_named(callee, "parallel") =>
            {
                let name = if is_named(callee, "coaxial") {
                    "coaxial"
                } else {
                    "parallel"
                };
                let (verdict, notes) = self.axis_condition(sc, name, args, e.span)?;
                Ok(Outcome { verdict, notes })
            }
            _ => Err(diag(
                "EVAL",
                e.span,
                "a check condition is a comparison or `x satisfies P`",
            )),
        }
    }

    /// Evaluates a comparison operand; an inexact value becomes an outer-bound interval so the
    /// comparison is undecided near the boundary (quantities.md 5.8).
    // frob:ticket 01M4V1HEA425D4RGFBSG5NT80Z
    fn operand(&mut self, sc: &mut Scope, e: &Expr) -> R<Operand> {
        match self.eval(sc, e)? {
            Val::Quantity(q) => Ok(Operand::Form(q.form)),
            Val::Inexact(i) => {
                debug!(lo = i.lo, hi = i.hi, "check: inexact comparison operand");
                Ok(Operand::Interval(
                    i.dim.clone(),
                    InexactInterval {
                        lo: i.lo,
                        hi: i.hi,
                        conservative: true,
                    },
                ))
            }
            _ => Err(diag("EVAL", e.span, "a quantity is required here")),
        }
    }

    fn compare_pair(&mut self, op: CmpOp, a: &Operand, b: &Operand, span: Span) -> R<Outcome> {
        let c = compare(op, a, b).map_err(|e| tol_diag(&e, span))?;
        for d in c.diagnostics() {
            if d.code == "C003" {
                self.diagnostics.push(EvalDiagnostic {
                    module: None,
                    code: "C003",
                    severity: Severity::Warning,
                    message: format!(
                        "{}; {}",
                        d.message,
                        d.fix.unwrap_or_else(|| "use the accessor form".into())
                    ),
                    span,
                });
            }
        }
        Ok(Outcome {
            verdict: c.verdict,
            notes: c.notes,
        })
    }

    /// Emits G005 when an undecided comparison read through a floated placement edge since
    /// `before` (geometry.md 5.2).
    // frob:ticket 01M4SWG70T71SNTZ2TWXZD1Y50
    fn note_float_read(&mut self, before: u32, verdict: Verdict, span: Span) {
        if verdict != Verdict::Undecided || self.float_reads == before {
            return;
        }
        warn!(
            ?span,
            "check: G005 check is tighter than a bolt float it reads through"
        );
        self.diagnostics.push(EvalDiagnostic {
            module: None,
            code: "G005",
            severity: Severity::Warning,
            message: "G005: this check is tighter than the float of a bolted joint it reads \
                      through; fix with `locating = true` and a fit/dowel, or loosen the check"
                .to_owned(),
            span,
        });
    }

    fn satisfies(&mut self, sc: &mut Scope, x: &Expr, p: &Expr, span: Span) -> R<Outcome> {
        let Some(def) = self.find_bound(sc, p, true) else {
            return Err(diag(
                "EVAL",
                p.span,
                "`satisfies` needs a bound on its right",
            ));
        };
        let bound = self.build_bound(&def, p.span)?;
        let before = self.float_reads;
        let x = self.quant(sc, x)?;
        let mut acc = Outcome::of(Verdict::Holds);
        for (op, limit) in [(CmpOp::Ge, bound.min()), (CmpOp::Le, bound.max())] {
            if let Some(m) = limit {
                let lim = Quant::constant(bound.dim().clone(), m.clone());
                let o = self.compare_pair(
                    op,
                    &Operand::Form(x.form.clone()),
                    &Operand::Form(lim.form.clone()),
                    span,
                )?;
                acc = join(acc, o, and);
            }
        }
        self.note_float_read(before, acc.verdict, span);
        Ok(acc)
    }

    /// Looks up the bound `e` names: `P` in this component or `Item.P`.
    fn find_bound(&self, sc: &Scope, e: &Expr, prefer_bound: bool) -> Option<BoundDef<'p>> {
        let (item, name) = match &strip(e).kind {
            ExprKind::Path(p) if p.segments.len() == 1 => {
                let c = sc.ctx.component.as_ref()?;
                (
                    CanonPath::item(&sc.ctx.package, &sc.ctx.module, c),
                    p.segments[0].name.clone(),
                )
            }
            ExprKind::Field(base, id) => match self.resolver.resolve_ref(&sc.ctx, base, None) {
                Ok(Resolved::Item { path, .. }) => (path, id.name.clone()),
                _ => return None,
            },
            _ => return None,
        };
        let mut key = item;
        key.members.push(MemberSeg::Name(name));
        if !prefer_bound && self.defs.contains_key(&key) {
            return None;
        }
        self.index.bounds.get(&key).cloned()
    }

    fn build_bound(&mut self, def: &BoundDef<'p>, span: Span) -> R<Bound> {
        let mut sc = Scope {
            ctx: def.ctx.clone(),
            locals: Vec::new(),
        };
        let mut limit = |this: &mut Self, e: &Expr| this.quant(&mut sc, e);
        match def.kind {
            BoundKind::Le { value, .. } => {
                let q = limit(self, value)?;
                Ok(Bound::at_most(q.dim().clone(), q.nominal))
            }
            BoundKind::Ge { value, .. } => {
                let q = limit(self, value)?;
                Ok(Bound::at_least(q.dim().clone(), q.nominal))
            }
            BoundKind::In { lo, hi, .. } => {
                let (l, h) = (limit(self, lo)?, limit(self, hi)?);
                l.dim()
                    .check_same_dimension(h.dim())
                    .map_err(|e| diag("Q002", span, e.to_string()))?;
                Bound::within(l.dim().clone(), l.nominal, h.nominal)
                    .map_err(|e| diag("EVAL", span, e.to_string()))
            }
            BoundKind::Eq { .. } => Err(diag("EVAL", span, "this bound has no limits")),
        }
    }

    /// `P.min` / `P.max` where `P` is a bound; `None` when `e` is not one.
    pub(super) fn bound_limit(&mut self, sc: &mut Scope, e: &Expr) -> Option<R<Val>> {
        let ExprKind::Field(base, id) = &e.kind else {
            return None;
        };
        if id.name != "min" && id.name != "max" {
            return None;
        }
        let def = self.find_bound(sc, base, false)?;
        Some(self.build_bound(&def, e.span).and_then(|b| {
            let limit = if id.name == "min" { b.min() } else { b.max() };
            limit
                .map(|m| Val::Quantity(Quant::constant(b.dim().clone(), m.clone())))
                .ok_or_else(|| {
                    diag(
                        "EVAL",
                        e.span,
                        format!("this bound has no `.{}` limit", id.name),
                    )
                })
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::tests::{file, run};

    fn verdicts(src: &str) -> (Vec<Verdict>, Vec<&'static str>, crate::eval::Evaluation) {
        let f = file(src);
        let ev = run(&f);
        let v = ev.checks.iter().map(|c| c.verdict).collect();
        let codes = ev.diagnostics.iter().map(|d| d.code).collect();
        (v, codes, ev)
    }

    // frob:tests check_false_everywhere_violates_c001
    #[test]
    fn check_false_everywhere_violates_c001() {
        let (v, codes, _) =
            verdicts("model M {\n  t = 5mm [+-0.1mm]\n  check low: t < 4mm \"too thick\"\n}\n");
        assert_eq!(v, vec![Verdict::Violates]);
        assert_eq!(codes, vec!["C001"]);
    }

    // frob:tests check_true_for_some_values_is_undecided_c002
    #[test]
    fn check_true_for_some_values_is_undecided_c002() {
        let (v, codes, _) = verdicts(
            "model M {\n  t = 5mm [+-0.1mm]\n  check edge: t > 5mm\n  check ok: t > 4mm\n}\n",
        );
        assert_eq!(v, vec![Verdict::Undecided, Verdict::Holds]);
        assert_eq!(codes, vec!["C002"]);
    }

    // frob:tests quantified_check_yields_one_verdict_per_instance
    #[test]
    fn quantified_check_yields_one_verdict_per_instance() {
        let (v, codes, ev) = verdicts(
            "model M {\n  a = 4mm\n  b = 9mm\n  check small for w in [a, b]: w < 5mm\n}\n",
        );
        assert_eq!(v, vec![Verdict::Holds, Verdict::Violates]);
        assert_eq!(codes, vec!["C001"]);
        assert_eq!(ev.checks[1].instance, Some(1));
    }

    // frob:tests satisfies_tests_value_against_bound_and_reads_limits
    #[test]
    fn satisfies_tests_value_against_bound_and_reads_limits() {
        let (v, codes, _) = verdicts(
            "ots V {\n  dia >= 9.6mm\n  span in 4mm..6mm\n}\n\
             model M {\n  d = 10mm\n  s = 7mm\n  check ok: d satisfies V.dia\n  \
             check bad: s satisfies V.span\n  check lim: V.dia.min == 9.6mm\n}\n",
        );
        assert_eq!(v, vec![Verdict::Holds, Verdict::Violates, Verdict::Holds]);
        assert_eq!(codes, vec!["C001"]);
    }

    // frob:tests require_is_a_check_with_citation_and_obligation
    #[test]
    fn require_is_a_check_with_citation_and_obligation() {
        let (v, _, ev) =
            verdicts("model M {\n  m = 5kg\n  require mass_budget: m <= 6kg @\"REQ-12\"\n}\n");
        assert_eq!(v, vec![Verdict::Holds]);
        let c = &ev.checks[0];
        assert!(c.obligation && c.citation.is_some());
    }

    // frob:tests check_path_is_canonical_member_path
    #[test]
    fn check_path_is_canonical_member_path() {
        let (_, _, ev) = verdicts("model M {\n  t = 5mm\n  check fine: t > 1mm\n}\n");
        assert_eq!(ev.checks[0].path.to_string(), "p::M.fine");
    }
}
