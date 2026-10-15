//! Processes: drawn versus designed values (language.md 3.8, quantities.md 5.6).
//!
//! A `process` declares parameters and `rule role(d) -> Kind = expr` items. A
//! member tagged `#[role]` in a component under `#![process(P)]` is
//! compensated: its *designed* value is the part as made (what fits, checks
//! and reads see) and its *drawn* value is the rule applied to the nominal
//! (what emit writes, what `x.drawn` reads). Use is two-phase because the
//! [`Evaluator`] is consumed by evaluation: [`Processes::compile`] runs first
//! (it evaluates process parameters and rule literals through
//! [`Evaluator::eval_input`]), then [`Processes::apply`] runs over the
//! finished [`Evaluation`].
//!
//! Decisions made here (written into language.md 3.8): a process is found by
//! name in the component's package first, then in any package; a component's
//! `#![process]` wins over the file's; rule bodies support literals, the rule
//! argument, process parameters, parentheses, negation and `+ - * /` (no
//! tolerance, call or match); the designed value keeps the member's nominal
//! and gains, for each toleranced process parameter a rule touches, that
//! parameter's deviation about its centre (the shared source variable); a
//! name is "feature-like" when one of its `_`-separated words is in
//! [`FEATURE_WORDS`].

// frob:ticket 01M4ST7APCCWC8KASSNHK0FW70

use std::collections::{BTreeMap, BTreeSet};

use cadetta_syntax::ast::{
    Attr, AttrArg, BinOp, Component, Expr, ExprKind, ItemKind, MemberKind, Process,
    ProcessMemberKind, Rule, UnOp,
};
use cadetta_syntax::span::Span;
use cadetta_units::{Affine, Dim, Magnitude, Sum};
use tracing::{debug, trace, warn};

use crate::deps::Severity;
use crate::eval::{EvalDiagnostic, Evaluation, Evaluator, Quant, Val};
use crate::resolve::{CanonPath, Ctx, MemberSeg, ModuleKey, PackageSrc};

/// Name words that make a parameter feature-like (P001).
pub const FEATURE_WORDS: &[&str] = &[
    "hole", "bore", "boss", "pin", "slot", "post", "pocket", "shaft", "stud", "standoff", "peg",
    "cutout", "rib",
];

/// A dimensioned exact number used while applying a rule.
#[derive(Clone, PartialEq, Eq, Debug)]
struct Nom {
    dim: Dim,
    mag: Magnitude,
}

/// A compiled rule body.
#[derive(Clone, PartialEq, Eq, Debug)]
enum RuleExpr {
    Const(Nom),
    Arg,
    Param(String),
    Neg(Box<RuleExpr>),
    Bin(BinOp, Box<RuleExpr>, Box<RuleExpr>),
}

/// A compiled `rule role(arg) -> Kind = expr`.
#[derive(Clone, PartialEq, Eq, Debug)]
struct RuleDef {
    body: RuleExpr,
    touched: BTreeSet<String>,
    span: Span,
}

/// A compiled process.
#[derive(Clone, PartialEq, Eq, Debug)]
struct ProcessDef {
    params: BTreeMap<String, Quant>,
    rules: BTreeMap<String, RuleDef>,
    /// The module that declares the process (where its rules' spans live).
    module: ModuleKey,
}

/// A member under a process with the role it plays.
#[derive(Clone, PartialEq, Eq, Debug)]
struct Plan {
    path: CanonPath,
    process: (String, String),
    role: String,
}

/// An analysis fact about processes (currently only P001).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ProcessFact {
    /// The lint code, `P001`.
    pub code: &'static str,
    /// The member concerned.
    pub path: CanonPath,
    /// The member's span.
    pub span: Span,
    /// The name of the process the member sits under.
    pub process: String,
    /// The human message.
    pub message: String,
}

/// A compensated member: designed (as made) and drawn (emitted) values.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Compensation {
    /// The process name.
    pub process: String,
    /// The role (rule) applied.
    pub role: String,
    /// The part as made: nominal plus the process scatter.
    pub designed: Quant,
    /// The rule applied to the nominal; carries no tolerance.
    pub drawn: Quant,
}

/// The result of [`Processes::apply`].
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct ProcessOutcome {
    /// Compensated members by path.
    pub compensated: BTreeMap<CanonPath, Compensation>,
    /// Problems found while applying rules.
    pub diagnostics: Vec<EvalDiagnostic>,
}

impl ProcessOutcome {
    /// What a read of `path` sees: the drawn value for `x.drawn`, else the designed one.
    pub fn read(&self, path: &CanonPath, drawn: bool) -> Option<&Quant> {
        let c = self.compensated.get(path)?;
        Some(if drawn { &c.drawn } else { &c.designed })
    }

    /// The quantity emit writes for `path` when it is compensated (the drawn value).
    pub fn emitted(&self, path: &CanonPath) -> Option<&Quant> {
        self.compensated.get(path).map(|c| &c.drawn)
    }
}

/// Compiled processes, the members they compensate and the P001 facts.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Processes {
    defs: BTreeMap<(String, String), ProcessDef>,
    plans: Vec<Plan>,
    /// P001 facts: feature-like parameters under a process with no role.
    pub facts: Vec<ProcessFact>,
    /// Problems found while compiling.
    pub diagnostics: Vec<EvalDiagnostic>,
}

fn problem(span: Span, message: impl Into<String>) -> EvalDiagnostic {
    let message = message.into();
    warn!(%message, "process error");
    EvalDiagnostic {
        module: None,
        code: "EVAL",
        severity: Severity::Error,
        message,
        span,
    }
}

fn exact(s: Sum, span: Span) -> Result<Magnitude, EvalDiagnostic> {
    match s {
        Sum::Exact(m) => Ok(m),
        Sum::Inexact(_) => Err(problem(span, "inexact sum in a process rule")),
    }
}

impl Nom {
    fn bin(&self, op: BinOp, o: &Self, span: Span) -> Result<Self, EvalDiagnostic> {
        match op {
            BinOp::Add | BinOp::Sub => {
                self.dim
                    .check_same_dimension(&o.dim)
                    .map_err(|e| problem(span, e.to_string()))?;
                let s = if op == BinOp::Add {
                    self.mag.add(&o.mag)
                } else {
                    self.mag.sub(&o.mag)
                };
                Ok(Self {
                    dim: self.dim.clone(),
                    mag: exact(s, span)?,
                })
            }
            BinOp::Mul => Ok(Self {
                dim: self.dim.mul(&o.dim),
                mag: self.mag.mul(&o.mag),
            }),
            BinOp::Div => Ok(Self {
                dim: self.dim.div(&o.dim),
                mag: self
                    .mag
                    .div(&o.mag)
                    .map_err(|e| problem(span, e.to_string()))?,
            }),
            _ => Err(problem(span, "operator not supported in a process rule")),
        }
    }
}

impl RuleExpr {
    fn eval(
        &self,
        arg: &Nom,
        params: &BTreeMap<String, Quant>,
        span: Span,
    ) -> Result<Nom, EvalDiagnostic> {
        match self {
            Self::Const(n) => Ok(n.clone()),
            Self::Arg => Ok(arg.clone()),
            Self::Param(p) => {
                let q = params
                    .get(p)
                    .ok_or_else(|| problem(span, format!("unknown process parameter `{p}`")))?;
                Ok(Nom {
                    dim: q.dim().clone(),
                    mag: q.nominal.clone(),
                })
            }
            Self::Neg(x) => {
                let n = x.eval(arg, params, span)?;
                Ok(Nom {
                    dim: n.dim,
                    mag: n.mag.neg(),
                })
            }
            Self::Bin(op, l, r) => {
                l.eval(arg, params, span)?
                    .bin(*op, &r.eval(arg, params, span)?, span)
            }
        }
    }
}

/// The single name of a one-segment path expression.
fn simple_name(e: &Expr) -> Option<&str> {
    match &e.kind {
        ExprKind::Path(p) if p.segments.len() == 1 => Some(p.segments[0].name.as_str()),
        _ => None,
    }
}

/// The process named by a `process(Name)` attribute, if `attrs` has one.
fn process_attr(attrs: &[Attr]) -> Option<String> {
    attrs.iter().rev().find_map(|a| {
        if a.name.name != "process" {
            return None;
        }
        match a.args.first() {
            Some(AttrArg::Expr(e)) => match &e.kind {
                ExprKind::Path(p) => p.segments.last().map(|s| s.name.clone()),
                _ => None,
            },
            _ => None,
        }
    })
}

/// True when a word of `name` says it is a feature (P001).
pub fn is_feature_like(name: &str) -> bool {
    name.split('_')
        .any(|w| FEATURE_WORDS.contains(&w.to_ascii_lowercase().as_str()))
}

impl Processes {
    /// Compiles every process and finds the members they compensate.
    pub fn compile<'r, 'p>(ev: &mut Evaluator<'r, 'p>, packages: &[PackageSrc<'p>]) -> Self {
        let mut out = Self::default();
        for p in packages {
            for m in &p.modules {
                let ctx = Ctx {
                    package: p.name.clone(),
                    module: m.path.clone(),
                    component: None,
                    locals: Vec::new(),
                };
                for item in m.file.items() {
                    if let ItemKind::Process(pr) = &item.kind {
                        let def = out.compile_process(ev, &ctx, pr);
                        debug!(process = %pr.name.name, rules = def.rules.len(), "process compiled");
                        out.defs.insert((p.name.clone(), pr.name.name.clone()), def);
                    }
                }
            }
        }
        for p in packages {
            for m in &p.modules {
                for c in m.file.components() {
                    out.plan_component(&p.name, &m.path, &m.file.inner_attrs, c);
                }
            }
        }
        out
    }

    fn compile_process<'r, 'p>(
        &mut self,
        ev: &mut Evaluator<'r, 'p>,
        ctx: &Ctx,
        pr: &Process,
    ) -> ProcessDef {
        let mut params = BTreeMap::new();
        for pm in &pr.members {
            if let ProcessMemberKind::Param(p) = &pm.kind {
                match ev.eval_input(ctx.clone(), &p.value) {
                    Ok(q) => {
                        params.insert(p.name.name.clone(), q);
                    }
                    Err(d) => self.diagnostics.push(d.in_module(ModuleKey::of_ctx(ctx))),
                }
            }
        }
        let mut rules = BTreeMap::new();
        for pm in &pr.members {
            if let ProcessMemberKind::Rule(r) = &pm.kind {
                match compile_rule(ev, ctx, r, &params) {
                    Ok(def) => {
                        rules.insert(r.name.name.clone(), def);
                    }
                    Err(d) => self.diagnostics.push(d.in_module(ModuleKey::of_ctx(ctx))),
                }
            }
        }
        ProcessDef {
            params,
            rules,
            module: ModuleKey::of_ctx(ctx),
        }
    }

    fn find(&self, package: &str, name: &str) -> Option<((String, String), &ProcessDef)> {
        let key = (package.to_owned(), name.to_owned());
        if let Some(d) = self.defs.get(&key) {
            return Some((key, d));
        }
        self.defs
            .iter()
            .find(|((_, n), _)| n == name)
            .map(|(k, d)| (k.clone(), d))
    }

    fn plan_component(
        &mut self,
        package: &str,
        module: &[String],
        file_inner: &[Attr],
        c: &Component,
    ) {
        let Some(name) = process_attr(&c.inner_attrs).or_else(|| process_attr(file_inner)) else {
            return;
        };
        if !c.params.is_empty() {
            trace!(component = %c.name.name, "process: parameterized component skipped");
            return;
        }
        let Some((key, def)) = self.find(package, &name) else {
            self.diagnostics.push(
                problem(c.name.span, format!("process `{name}` is not declared"))
                    .in_module(ModuleKey::new(package, module)),
            );
            return;
        };
        let rule_names: BTreeSet<String> = def.rules.keys().cloned().collect();
        let item = CanonPath::item(package, module, &c.name.name);
        let mut plans = Vec::new();
        let mut facts = Vec::new();
        for m in &c.members {
            let MemberKind::Param(p) = &m.kind else {
                continue;
            };
            let mut path = item.clone();
            path.members.push(MemberSeg::Name(p.name.name.clone()));
            let role = m
                .attrs
                .iter()
                .chain(m.trailing.iter())
                .find(|a| a.args.is_empty() && rule_names.contains(&a.name.name))
                .map(|a| a.name.name.clone());
            match role {
                Some(role) => plans.push(Plan {
                    path,
                    process: key.clone(),
                    role,
                }),
                None if is_feature_like(&p.name.name) => {
                    debug!(member = %p.name.name, "process: P001 missing role");
                    facts.push(ProcessFact {
                        code: "P001",
                        path,
                        span: p.name.span,
                        process: name.to_string(),
                        message: format!(
                            "`{}` looks like a feature but has no role under process `{name}`",
                            p.name.name
                        ),
                    });
                }
                None => trace!(member = %p.name.name, "process: member not compensated"),
            }
        }
        self.plans.extend(plans);
        self.facts.extend(facts);
    }

    /// Applies `role` of `process` (package, name) to an emit `point`, returning the drawn value.
    pub fn drawn_at(
        &self,
        process: &(String, String),
        role: &str,
        point: &Quant,
    ) -> Result<Quant, EvalDiagnostic> {
        let span = Span::default();
        let def = self
            .defs
            .get(process)
            .ok_or_else(|| problem(span, format!("process `{}` is not compiled", process.1)))?;
        let rule = def.rules.get(role).ok_or_else(|| {
            problem(
                span,
                format!("process `{}` has no rule `{role}`", process.1),
            )
        })?;
        let arg = Nom {
            dim: point.dim().clone(),
            mag: point.nominal.clone(),
        };
        let n = rule.body.eval(&arg, &def.params, rule.span)?;
        n.dim
            .check_same_dimension(&arg.dim)
            .map_err(|e| problem(rule.span, format!("rule `{role}` changes the kind: {e}")))?;
        Ok(Quant::constant(n.dim, n.mag))
    }

    /// The drawn value of member `path` whose designed value is `q`, or `None` when it has no role.
    // frob:ticket 01M4Z33NNYYPECFPASHMRQXAZQ
    pub fn drawn_of(&self, path: &CanonPath, q: &Quant) -> Option<Result<Quant, EvalDiagnostic>> {
        let plan = self.plans.iter().find(|p| &p.path == path)?;
        trace!(?path, role = %plan.role, "process: drawn value read");
        Some(self.drawn_at(&plan.process, &plan.role, q))
    }

    /// Applies every planned compensation to the evaluated members.
    pub fn apply(&self, evaluation: &Evaluation) -> ProcessOutcome {
        let mut out = ProcessOutcome::default();
        for plan in &self.plans {
            let Some(ev) = evaluation.get(&plan.path) else {
                trace!(path = ?plan.path, "process: member did not evaluate");
                continue;
            };
            let Val::Quantity(q) = &ev.value else {
                continue;
            };
            if ev.analysis {
                continue;
            }
            let Some(def) = self.defs.get(&plan.process) else {
                continue;
            };
            let Some(rule) = def.rules.get(&plan.role) else {
                continue;
            };
            match self.compensate(plan, def, rule, q) {
                Ok(c) => {
                    debug!(path = ?plan.path, role = %plan.role, "process: member compensated");
                    out.compensated.insert(plan.path.clone(), c);
                }
                Err(d) => out.diagnostics.push(d.in_module(def.module.clone())),
            }
        }
        out
    }

    fn compensate(
        &self,
        plan: &Plan,
        def: &ProcessDef,
        rule: &RuleDef,
        q: &Quant,
    ) -> Result<Compensation, EvalDiagnostic> {
        let drawn = self.drawn_at(&plan.process, &plan.role, q)?;
        let mut form = q.form.clone();
        for name in &rule.touched {
            let Some(p) = def.params.get(name) else {
                continue;
            };
            let centre = Affine::constant(p.dim().clone(), p.form.center().clone());
            let dev = p
                .form
                .sub(&centre)
                .map_err(|e| problem(rule.span, e.to_string()))?;
            form = form
                .add(&dev)
                .map_err(|e| problem(rule.span, e.to_string()))?;
        }
        Ok(Compensation {
            process: plan.process.1.clone(),
            role: plan.role.clone(),
            designed: Quant {
                form,
                nominal: q.nominal.clone(),
            },
            drawn,
        })
    }
}

fn compile_rule<'r, 'p>(
    ev: &mut Evaluator<'r, 'p>,
    ctx: &Ctx,
    r: &Rule,
    params: &BTreeMap<String, Quant>,
) -> Result<RuleDef, EvalDiagnostic> {
    let mut touched = BTreeSet::new();
    let body = compile_expr(ev, ctx, &r.body, &r.param.name.name, params, &mut touched)?;
    Ok(RuleDef {
        body,
        touched,
        span: r.name.span,
    })
}

fn compile_expr<'r, 'p>(
    ev: &mut Evaluator<'r, 'p>,
    ctx: &Ctx,
    e: &Expr,
    arg: &str,
    params: &BTreeMap<String, Quant>,
    touched: &mut BTreeSet<String>,
) -> Result<RuleExpr, EvalDiagnostic> {
    match &e.kind {
        ExprKind::Number(_) | ExprKind::Quantity(_) => {
            let q = ev.eval_input(ctx.clone(), e)?;
            Ok(RuleExpr::Const(Nom {
                dim: q.dim().clone(),
                mag: q.nominal,
            }))
        }
        ExprKind::Path(_) => match simple_name(e) {
            Some(n) if n == arg => Ok(RuleExpr::Arg),
            Some(n) if params.contains_key(n) => {
                touched.insert(n.to_owned());
                Ok(RuleExpr::Param(n.to_owned()))
            }
            _ => Err(problem(
                e.span,
                "a process rule may read only its argument and process parameters",
            )),
        },
        ExprKind::Paren(x) => compile_expr(ev, ctx, x, arg, params, touched),
        ExprKind::Unary(UnOp::Neg, x) => Ok(RuleExpr::Neg(Box::new(compile_expr(
            ev, ctx, x, arg, params, touched,
        )?))),
        ExprKind::Binary(op @ (BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div), l, r) => {
            Ok(RuleExpr::Bin(
                *op,
                Box::new(compile_expr(ev, ctx, l, arg, params, touched)?),
                Box::new(compile_expr(ev, ctx, r, arg, params, touched)?),
            ))
        }
        _ => Err(problem(
            e.span,
            "this construct is not supported in a process rule",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolve::{ModuleSrc, Resolver};
    use cadetta_syntax::parser::parse;
    use cadetta_units::{Rational, UnitTable, VarId};

    const SRC: &str = "process Fdm @measured:\"calibration print\" {\n  \
        hole_comp = 0.2mm [+-0.05mm]\n  \
        rule hole(d: Length) -> Length = d + hole_comp\n  \
        rule boss(d: Length) -> Length = d - 0.1mm\n}\n\
        model Case {\n  #![process(Fdm)]\n  insert_hole = 3.4mm [+-0.1mm] #[hole]\n  \
        peg = 5mm #[boss]\n  slot_width = 2mm\n  wall = 3mm\n}\n\
        model Plain {\n  insert_hole = 3.4mm\n}\n";

    fn run(src: &str) -> (Processes, Evaluation, ProcessOutcome) {
        let p = parse(src);
        assert!(p.diags.is_empty(), "{:?}", p.diags);
        let pkgs = vec![PackageSrc {
            name: "p".to_owned(),
            deps: Vec::new(),
            modules: vec![ModuleSrc {
                path: Vec::new(),
                file: &p.file,
                enums: &[],
            }],
        }];
        let (resolver, errs) = Resolver::new(pkgs.clone());
        assert!(errs.is_empty());
        let units = UnitTable::builtin();
        let mut ev = Evaluator::new(&resolver, &units, &pkgs);
        let procs = Processes::compile(&mut ev, &pkgs);
        let evaluation = ev.evaluate();
        let outcome = procs.apply(&evaluation);
        (procs, evaluation, outcome)
    }

    fn at(comp: &str, member: &str) -> CanonPath {
        let mut p = CanonPath::item("p", &[], comp);
        p.members.push(MemberSeg::Name(member.to_owned()));
        p
    }

    fn mm(n: i64, d: i64) -> Magnitude {
        Magnitude::new(Rational::new(n, d).expect("denominator"), 0)
    }

    // frob:tests crates/cadetta-sema/src/process.rs::Processes.apply kind=unit
    #[test]
    fn drawn_is_rule_on_nominal_and_designed_is_as_made() {
        let (procs, ev, out) = run(SRC);
        assert!(procs.diagnostics.is_empty(), "{:?}", procs.diagnostics);
        assert!(out.diagnostics.is_empty(), "{:?}", out.diagnostics);
        let c = &out.compensated[&at("Case", "insert_hole")];
        assert_eq!(c.drawn.nominal, mm(36, 10_000));
        assert!(c.drawn.form.is_exact());
        assert_eq!(c.designed.nominal, mm(34, 10_000));
        let boss = &out.compensated[&at("Case", "peg")];
        assert_eq!(boss.drawn.nominal, mm(49, 10_000));
        // The evaluation itself is untouched: checks and fits see the designed nominal.
        let Val::Quantity(q) = &ev.get(&at("Case", "insert_hole")).expect("ev").value else {
            panic!("quantity")
        };
        assert_eq!(q.nominal, c.designed.nominal);
        assert_eq!(out.read(&at("Case", "insert_hole"), true), Some(&c.drawn));
        assert_eq!(
            out.read(&at("Case", "insert_hole"), false),
            Some(&c.designed)
        );
        assert_eq!(out.emitted(&at("Case", "peg")), Some(&boss.drawn));
    }

    // frob:tests crates/cadetta-sema/src/process.rs::Processes.apply kind=unit
    #[test]
    fn process_parameter_scatter_is_added_to_designed() {
        let (_, ev, out) = run(SRC);
        let c = &out.compensated[&at("Case", "insert_hole")];
        let Val::Quantity(q) = &ev.get(&at("Case", "insert_hole")).expect("ev").value else {
            panic!("quantity")
        };
        let (lo, hi) = q.form.bounds().expect("bounds");
        let (dlo, dhi) = c.designed.form.bounds().expect("bounds");
        // +-0.1 widens by the process's +-0.05: +-0.15.
        assert_eq!(dlo, mm(325, 100_000));
        assert_eq!(dhi, mm(355, 100_000));
        assert_eq!((lo, hi), (mm(33, 10_000), mm(35, 10_000)));
        // The scatter is a source variable of its own, not the member's.
        assert!((0..8).any(|i| c.designed.form.coefficient(VarId(i)).is_some()));
        // A rule that touches no process parameter adds no scatter.
        let b = &out.compensated[&at("Case", "peg")];
        assert!(b.designed.form.is_exact());
    }

    // frob:tests crates/cadetta-sema/src/process.rs::Processes.compile kind=unit
    #[test]
    fn member_without_role_is_not_compensated_and_p001_is_recorded() {
        let (procs, _, out) = run(SRC);
        assert!(!out.compensated.contains_key(&at("Case", "slot_width")));
        assert!(!out.compensated.contains_key(&at("Case", "wall")));
        assert!(!out.compensated.contains_key(&at("Plain", "insert_hole")));
        let codes: Vec<_> = procs
            .facts
            .iter()
            .map(|f| (f.code, f.path.clone()))
            .collect();
        assert_eq!(codes, vec![("P001", at("Case", "slot_width"))]);
    }

    // frob:tests crates/cadetta-sema/src/process.rs::Processes.compile kind=unit
    #[test]
    fn roles_are_not_inherited_from_scope() {
        let src = "process P { rule hole(d: Length) -> Length = d + 1mm }\n\
                   model M {\n  #![process(P)]\n  #![hole]\n  a = 3mm\n}\n";
        let (_, _, out) = run(src);
        assert!(out.compensated.is_empty());
    }

    // frob:tests crates/cadetta-sema/src/process.rs::Processes.drawn_at kind=unit
    #[test]
    fn emit_applies_rule_to_chosen_point() {
        let (procs, ev, _) = run(SRC);
        let Val::Quantity(q) = &ev.get(&at("Case", "insert_hole")).expect("ev").value else {
            panic!("quantity")
        };
        let key = ("p".to_owned(), "Fdm".to_owned());
        let mid = Quant::constant(q.dim().clone(), mm(30, 10_000));
        let d = procs.drawn_at(&key, "hole", &mid).expect("drawn");
        assert_eq!(d.nominal, mm(32, 10_000));
        assert!(procs.drawn_at(&key, "nope", &mid).is_err());
    }

    // frob:tests crates/cadetta-sema/src/process.rs::Processes.compile kind=unit
    #[test]
    fn unsupported_rule_construct_is_reported() {
        let src = "process P { rule hole(d: Length) -> Length = sqrt(d) }\n";
        let (procs, _, _) = run(src);
        assert_eq!(procs.diagnostics.len(), 1);
    }

    #[test]
    fn feature_words() {
        assert!(is_feature_like("insert_hole"));
        assert!(is_feature_like("Bore_dia"));
        assert!(!is_feature_like("wall"));
        assert!(!is_feature_like("holder"));
    }
}
