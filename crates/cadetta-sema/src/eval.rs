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
//! Parameterized components are evaluated only as instances (a later ticket).
//! Inexact results (`sqrt`, trigonometry) are carried as `func::Inex` values:
//! a binary64 nominal with a conservative interval (quantities.md 5.3, 5.4).

// frob:ticket 01M4SPWPPPSEEFAHYTY5H3YD54
// frob:ticket 01M4ST7ASWYQV13AAHHCXVR570
// frob:ticket 01M4ST7AFZGCN52434BQJRT5MM

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::rc::Rc;

use cadetta_syntax::ast::{
    self, BinOp, Block, Component, Expr, ExprKind, FnDecl, ItemKind, MemberKind, TolSign, TolTerm,
    Type, UnOp,
};
use cadetta_syntax::lexer::Segment;
use cadetta_syntax::span::Span;
use cadetta_units::square::{mul_checked, square};
use cadetta_units::{
    Affine, Dim, InexactError, Magnitude, Rational, Role, RoleInfo, RoleSource, Sum, Temperature,
    TemperatureUnit, ToleranceError, Toleranced, UnitTable, VarId, resolve_role,
};
use tracing::{debug, trace, warn};

use crate::deps::Severity;
use crate::resolve::{
    CanonPath, Ctx, ItemClass, Level, MemberSeg, ModuleKey, PackageSrc, Resolved, Resolver,
};

#[path = "array.rs"]
pub mod array;
#[path = "boltfloat.rs"]
mod boltfloat;
#[path = "checks.rs"]
pub mod checks;
#[path = "enums.rs"]
pub mod enums;
#[path = "func.rs"]
pub mod func;
#[path = "geom.rs"]
pub mod geom;
#[path = "instances.rs"]
mod instances;
#[path = "point.rs"]
pub mod point;
#[path = "posshape.rs"]
mod posshape;
#[path = "tablevals.rs"]
mod tablevals;

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
    /// An enum variant.
    Variant {
        /// The enum.
        enum_path: CanonPath,
        /// The variant name.
        variant: String,
    },
    /// An array of values.
    Array(Vec<Val>),
    /// A `Point2`/`Point3` (or `Vec2`/`Vec3`): two or three Length components.
    Point(Vec<Val>),
    /// An inexact number (`sqrt`, trigonometry): interval plus root operation.
    Inexact(func::Inex),
    /// A table named as a value (a policy member such as `drills = IsoDrills`).
    Table(CanonPath),
    /// One row of a table, read by `.key` or by column name.
    Row {
        /// The table.
        table: CanonPath,
        /// The row's position in source order.
        row: usize,
    },
    /// A component named as a value: an `ots` policy argument or a part instance.
    // frob:ticket 01M4W3YJCH948JJQ3HY646D1T7
    Component(CanonPath),
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
    /// The hole/shaft role (quantities.md 5.6): explicit, from a class letter, or inherited.
    pub role: Option<RoleInfo>,
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
    /// The module the primary span lies in, when known.
    pub module: Option<ModuleKey>,
}

impl<'r, 'p> Evaluator<'r, 'p> {
    /// Stamps `module` on every diagnostic recorded since index `start` that has none.
    pub(super) fn stamp_since(&mut self, start: usize, module: &ModuleKey) {
        for d in &mut self.diagnostics[start..] {
            if d.module.is_none() {
                d.module = Some(module.clone());
            }
        }
    }
}

impl EvalDiagnostic {
    /// Records `module` as the span's module unless one is already set.
    pub fn in_module(mut self, module: ModuleKey) -> Self {
        if self.module.is_none() {
            self.module = Some(module);
        }
        self
    }
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
    /// The verdict of every `check` and `require`, in source order.
    pub checks: Vec<checks::CheckResult>,
    /// How many source variables were allocated (ids `0..var_count`); lowering enumerates them.
    pub var_count: u32,
    /// The evaluated instance count of every counted part slot (`part leg[4]`), by part path.
    pub part_counts: BTreeMap<CanonPath, u64>,
    /// Instance members that are `priv` in their component (lowering marks them private).
    // frob:ticket 01M4W3YJCH948JJQ3HY646D1T7
    pub instance_private: BTreeSet<CanonPath>,
    /// Datum and placement members evaluated per parameterized instance (`Box/j.mount.offset.x`).
    // frob:ticket 01M4VYZS8VGT1RASS407Y8K809
    pub instance_geometry: BTreeMap<CanonPath, GeometryKind>,
}

/// Which geometry declaration an instance geometry member belongs to.
// frob:ticket 01M4VYZS8VGT1RASS407Y8K809
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GeometryKind {
    /// An operation argument of a `datum`.
    Datum,
    /// An operation argument of a `place`.
    Place,
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

impl Fail {
    /// Stamps the module on a carried diagnostic that has none.
    fn in_module(self, module: ModuleKey) -> Self {
        match self {
            Self::Diag(d) => Self::Diag(d.in_module(module)),
            Self::Silent => Self::Silent,
        }
    }
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
        module: None,
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
    // frob:ticket 01M4V1HDVGKT6AVWCWY7078TNV
    fn accessor(&self, name: &str, role: Option<Role>, span: Span) -> R<Self> {
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
                let Some(role) = role else {
                    return Err(diag(
                        "E010",
                        span,
                        format!(
                            "`.{name}` needs a role (hole or shaft) to say which side is material"
                        ),
                    ));
                };
                let (lo, hi) = self.form.bounds().map_err(|e| tol_diag(&e, span))?;
                // A hole's maximum material is its smallest size; a shaft's its largest.
                let smallest = (name == "mmc") == (role == Role::Hole);
                if smallest { lo } else { hi }
            }
        };
        Ok(Self::constant(dim, m))
    }
}

/// How a reference expression resolved.
enum RefKind {
    /// A member of a definition (or an instance), then the names read from its value
    /// (an accessor, a component member, a row column, a point coordinate).
    Member { key: CanonPath, chain: Vec<String> },
    /// A component or part instance named as a value.
    Component(CanonPath),
    /// A block-local binding.
    Local(String),
    /// An enum variant.
    Variant {
        enum_path: CanonPath,
        variant: String,
    },
    /// A table named as a value.
    Table(CanonPath),
}

/// A member definition indexed for evaluation.
#[derive(Clone)]
struct Def<'p> {
    expr: &'p Expr,
    ty: Option<&'p Type>,
    is_let: bool,
    ctx: Ctx,
    span: Span,
    /// The instance this definition belongs to (callee members and callee-scope defaults).
    inst: Option<Rc<instances::InstInfo>>,
    /// A value known without evaluating `expr` (a bare enum variant or a default policy).
    fixed: Option<Val>,
    /// The `#[when(cond)]` presence condition, evaluated in `ctx`.
    when: Option<&'p Expr>,
    /// The explicit `#[role(hole|shaft)]` (or bare `#[hole]`/`#[shaft]`) of the member.
    role: Option<RoleInfo>,
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

/// One slot swap: `alt` replaces the primary of `owner`'s part slot `slot` in every instance
/// (language.md 3.2).
// frob:ticket 01M4TRMJMNWCMHCE2H3BJF002X
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Selection {
    /// The component that declares the slot.
    pub owner: CanonPath,
    /// The slot name.
    pub slot: String,
    /// The alternate component standing in for the slot's primary.
    pub alt: CanonPath,
}

impl Selection {
    /// The full-form variant key `PKG::Owner/slot=PKG::Alt` (ir.md 3).
    pub fn key(&self) -> String {
        format!("{}/{}={}", self.owner, self.slot, self.alt)
    }
}

/// Evaluates the parameters and `let`s of a project, on demand and memoised.
pub struct Evaluator<'r, 'p> {
    resolver: &'r Resolver<'p>,
    /// Every `was` of the project, so a stale reference reads the current member (N004 is the graph's).
    renames: crate::deps::Renames,
    units: &'r UnitTable,
    defs: BTreeMap<CanonPath, Def<'p>>,
    /// Components whose own datums and placements are evaluated (those without parameters;
    /// parameterized ones are evaluated per instance, see `instance_geometry`).
    geom_components: Vec<CanonPath>,
    /// Instance counts of evaluated `part name[N]` slots, by part path.
    part_counts: BTreeMap<CanonPath, u64>,
    /// Geometry members evaluated per instance, by instance path.
    // frob:ticket 01M4VYZS8VGT1RASS407Y8K809
    instance_geometry: BTreeMap<CanonPath, GeometryKind>,
    /// Part nodes of the current component whose placement raised G004 (not expanded).
    g004_nodes: BTreeSet<String>,
    done: BTreeMap<CanonPath, Option<Evaluated>>,
    /// The slot swap reads through part slots follow; `None` selects every primary.
    selection: Option<Selection>,
    in_progress: BTreeSet<CanonPath>,
    next_var: u32,
    /// The x, y and rotation source variables of each bolt-float joint (geometry.md 5.2).
    float_vars: BTreeMap<CanonPath, [VarId; 3]>,
    /// Count of datum re-expressions that read through a floated placement edge (G005).
    float_reads: u32,
    /// Joints already reported G008, so each warns once.
    g008_seen: BTreeSet<String>,
    diagnostics: Vec<EvalDiagnostic>,
    index: checks::CheckIndex<'p>,
    /// Functions on a call-graph cycle (language.md 3.8: recursion is rejected).
    recursive: BTreeSet<CanonPath>,
    /// Every `table` item of the project, by path.
    tables: BTreeMap<CanonPath, &'p ast::Table>,
    /// Tables built so far (built on first read).
    table_cache: BTreeMap<CanonPath, std::rc::Rc<tablevals::TableVals>>,
    /// Instances of parameterized components, materialised from `part` members.
    instances: instances::Instances<'p>,
    /// The instance whose member is being evaluated (callee names map to its prefix).
    cur_inst: Option<Rc<instances::InstInfo>>,
    /// Instance members whose `#[when]` condition was false.
    absent: BTreeSet<CanonPath>,
    /// Roles implied by `fit` members, for members with no explicit or class role.
    fit_roles: BTreeMap<CanonPath, Role>,
    /// Every `fit` member of a component without parameters, by path.
    // frob:ticket 01M4Z33NKSMBJSGKAMVDGYN4CP
    fit_defs: BTreeMap<CanonPath, FitDef<'p>>,
    /// Fit clearances computed so far (`None` when the fit could not be analysed).
    fit_done: BTreeMap<CanonPath, Option<Quant>>,
    /// Compiled processes, so a check can read `x.drawn` (language.md 3.8).
    // frob:ticket 01M4Z33NNYYPECFPASHMRQXAZQ
    processes: Option<Rc<crate::process::Processes>>,
}

/// A `fit` member with the scope its operands resolve in.
// frob:ticket 01M4Z33NKSMBJSGKAMVDGYN4CP
#[derive(Clone)]
struct FitDef<'p> {
    fit: &'p ast::Fit,
    ctx: Ctx,
}

impl<'r, 'p> Evaluator<'r, 'p> {
    /// Indexes every parameter and `let` of every component without parameters.
    pub fn new(
        resolver: &'r Resolver<'p>,
        units: &'r UnitTable,
        packages: &[PackageSrc<'p>],
    ) -> Self {
        let mut defs = BTreeMap::new();
        let mut fit_defs = BTreeMap::new();
        let mut geom_components = Vec::new();
        let mut index = checks::CheckIndex::default();
        let mut fns: BTreeMap<CanonPath, &'p FnDecl> = BTreeMap::new();
        let mut tables: BTreeMap<CanonPath, &'p ast::Table> = BTreeMap::new();
        for p in packages {
            for m in &p.modules {
                for item in m.file.items() {
                    if let ItemKind::Fn(f) = &item.kind {
                        fns.insert(CanonPath::item(&p.name, &m.path, &f.name.name), f);
                    }
                    if let ItemKind::Table(t) = &item.kind {
                        tables.insert(CanonPath::item(&p.name, &m.path, &t.name.name), t);
                    }
                }
                for c in m.file.components() {
                    index_component(&mut defs, &p.name, &m.path, c);
                    index_fits(&mut fit_defs, &p.name, &m.path, c);
                    if c.params.is_empty() {
                        geom_components.push(CanonPath::item(&p.name, &m.path, &c.name.name));
                    }
                    index.add_component(&p.name, &m.path, c);
                }
            }
        }
        let instances = instances::materialise(resolver, packages, &mut defs);
        debug!(members = defs.len(), "evaluator: indexed members");
        let mut diagnostics = instances.diagnostics.clone();
        let mut recursive = BTreeSet::new();
        for (path, span) in Self::find_recursive(resolver, &fns) {
            diagnostics.push(EvalDiagnostic {
                code: "EVAL",
                severity: Severity::Error,
                message: format!("function `{path}` is recursive; recursion is not allowed"),
                span,
                module: Some(ModuleKey::new(&path.package, &path.modules)),
            });
            recursive.insert(path);
        }
        Self {
            resolver,
            renames: crate::deps::Renames::collect(resolver, packages),
            units,
            defs,
            geom_components,
            part_counts: BTreeMap::new(),
            instance_geometry: BTreeMap::new(),
            g004_nodes: BTreeSet::new(),
            done: BTreeMap::new(),
            selection: None,
            in_progress: BTreeSet::new(),
            next_var: 0,
            float_vars: BTreeMap::new(),
            float_reads: 0,
            g008_seen: BTreeSet::new(),
            diagnostics,
            index,
            recursive,
            tables,
            table_cache: BTreeMap::new(),
            instances,
            cur_inst: None,
            absent: BTreeSet::new(),
            fit_roles: crate::fitderive::implied_roles(resolver, packages),
            fit_defs,
            fit_done: BTreeMap::new(),
            processes: None,
        }
    }

    /// This evaluator under `selection`: reads through the swapped slot reach the alternate.
    // frob:ticket 01M4TRMJMNWCMHCE2H3BJF002X
    #[must_use]
    pub fn with_selection(mut self, selection: Selection) -> Self {
        debug!(variant = %selection.key(), "evaluator: slot selection set");
        self.selection = Some(selection);
        self
    }

    /// Evaluates one free-standing expression (a solver `in` value) in `ctx`.
    // frob:ticket 01M4STAN8QG3QWZQCH5XN3QHMT
    pub fn eval_input(&mut self, ctx: Ctx, e: &Expr) -> Result<Quant, EvalDiagnostic> {
        let here = ModuleKey::of_ctx(&ctx);
        let mut sc = Scope {
            ctx,
            locals: Vec::new(),
        };
        match self.quant(&mut sc, e) {
            Ok(q) => Ok(q),
            Err(Fail::Diag(d)) => Err(d.in_module(here)),
            Err(Fail::Silent) => Err(EvalDiagnostic {
                module: Some(here),
                code: "EVAL",
                severity: Severity::Error,
                message: "a value this expression reads failed to evaluate".to_owned(),
                span: e.span,
            }),
        }
    }

    /// Evaluates every indexed member and returns the values and diagnostics.
    pub fn evaluate(mut self) -> Evaluation {
        let keys: Vec<CanonPath> = self.defs.keys().cloned().collect();
        for k in &keys {
            self.value_of(k);
        }
        self.check_requires();
        self.evaluate_geometry();
        let checks = self.run_checks();
        self.drop_shadowed_evals();
        let instance_private = self.instances.private.clone();
        let instance_geometry = std::mem::take(&mut self.instance_geometry);
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
            checks,
            var_count: self.next_var,
            part_counts: self.part_counts,
            instance_private,
            instance_geometry,
        }
    }

    /// Drops the internal `EVAL` failures that sit inside a span already reported as `G001`/`G003`.
    // frob:ticket 01M4V436F7JRA8P832RH0EKNH2
    fn drop_shadowed_evals(&mut self) {
        let spans: Vec<Span> = self
            .diagnostics
            .iter()
            .filter(|d| matches!(d.code, "G001" | "G003"))
            .map(|d| d.span)
            .collect();
        self.diagnostics.retain(|d| {
            !(d.code == "EVAL"
                && spans
                    .iter()
                    .any(|s| s.start <= d.span.start && d.span.end <= s.end))
        });
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
        let saved = std::mem::replace(&mut self.cur_inst, def.inst.clone());
        let present = match def.when {
            Some(cond) => match self.eval(&mut scope, cond) {
                Ok(Val::Bool(b)) => Ok(b),
                Ok(_) => Err(diag("EVAL", cond.span, "a `when` condition must be a Bool")),
                Err(f) => Err(f),
            },
            None => Ok(true),
        };
        let result = match present {
            Ok(true) => self.eval_def(&mut scope, &def, key),
            Ok(false) => {
                debug!(%key, "evaluator: `when` is false, member absent");
                self.absent.insert(key.clone());
                Err(Fail::Silent)
            }
            Err(f) => Err(f),
        };
        self.cur_inst = saved;
        self.in_progress.remove(key);
        let out = match result {
            Ok(v) => Some(v),
            Err(Fail::Silent) => None,
            Err(Fail::Diag(d)) => {
                self.diagnostics
                    .push(d.in_module(ModuleKey::of_ctx(&def.ctx)));
                None
            }
        };
        self.done.insert(key.clone(), out.clone());
        out
    }

    /// Evaluates one present definition, then checks it against its declared type.
    fn eval_def(&mut self, scope: &mut Scope, def: &Def<'p>, key: &CanonPath) -> R<Evaluated> {
        let result = if let Some(v) = &def.fixed {
            Ok(Evaluated {
                value: v.clone(),
                analysis: false,
                var: None,
                alias_of: None,
                span: def.span,
                role: None,
            })
        } else if def.is_let {
            self.eval_let(scope, def)
        } else {
            self.eval_param(scope, def, key)
        };
        // An explicit `#[role(..)]` wins over a class-letter or inherited role.
        let result = result.map(|mut ev| {
            ev.role = def.role.or(ev.role).or_else(|| {
                self.fit_roles.get(key).map(|&role| RoleInfo {
                    role,
                    source: RoleSource::FitSide,
                })
            });
            ev
        });
        match (result, def.ty) {
            (Ok(ev), Some(ty)) => self
                .check_declared(scope, ty, &ev.value, def.span)
                .map(|()| ev),
            (r, _) => r,
        }
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
            role: None,
        })
    }

    fn eval_param(&mut self, sc: &mut Scope, def: &Def<'p>, _key: &CanonPath) -> R<Evaluated> {
        self.eval_param_expr(sc, def.expr, def.span)
    }

    /// Evaluates `expr` the way a parameter value is: tolerance and range own a fresh variable.
    // frob:ticket 01M4VQXPDA4TEKE455FNYB5TXC
    fn eval_param_expr(&mut self, sc: &mut Scope, expr: &Expr, span: Span) -> R<Evaluated> {
        let e = strip(expr);
        // A pure reference is an alias of its referent's source variable.
        if matches!(e.kind, ExprKind::Path(_) | ExprKind::Field(..))
            && let Ok(RefKind::Member { key: rk, chain }) = self.classify(sc, e)
            && chain.is_empty()
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
                role: resolve_role(None, ev.role),
            });
        }
        match &e.kind {
            ExprKind::Tolerance(inner, terms) => {
                let (v, var) = self.tolerance_val(sc, inner, terms, e.span)?;
                Ok(owned(v, var, span))
            }
            ExprKind::Binary(BinOp::Range, l, r) => {
                let var = self.fresh_var();
                let q = self.range(sc, l, r, var, e.span)?;
                Ok(owned(Val::Quantity(q), Some(var), span))
            }
            // frob:ticket 01M4V1HDVGKT6AVWCWY7078TNV
            ExprKind::IsoClass(inner, class) => {
                let var = self.fresh_var();
                let (q, role) = self.iso_class(sc, inner, class, var, e.span)?;
                let mut ev = owned(Val::Quantity(q), Some(var), span);
                ev.role = Some(role);
                Ok(ev)
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
        // frob:ticket 01M4YR00ZFABJQWKQJ3F3JFVY4
        let resolved = self
            .renames
            .resolve_ref(self.resolver, &sc.ctx, e, None)
            .map(|(r, _)| r)
            .map_err(|re| {
                Fail::Diag(EvalDiagnostic {
                    code: re.code,
                    severity: Severity::Error,
                    message: re.message,
                    span: re.span,
                    module: re.module.map(|m| *m),
                })
            })?;
        match resolved {
            Resolved::Local(n) => Ok(RefKind::Local(n)),
            // frob:ticket 01M4W3YJCH948JJQ3HY646D1T7
            Resolved::Arg { path, .. } => match self.instance_path(&path) {
                Some(p) => member_ref(p, e.span),
                None => Err(diag(
                    "EVAL",
                    e.span,
                    "a component argument is read only inside its instance",
                )),
            },
            Resolved::Member { path, level, .. } => {
                let path = if let Some(p) = self.instance_path(&path) {
                    p
                } else if path.parts.is_empty() {
                    if level == Level::Instance {
                        return Err(diag(
                            "EVAL",
                            e.span,
                            "reading a part instance is evaluated by the parts ticket",
                        ));
                    }
                    path
                } else if self.defs.contains_key(&first_member(&path)) {
                    // frob:ticket 01M4W3YJCH948JJQ3HY646D1T7
                    path
                } else {
                    // frob:ticket 01M4VQXPDA4TEKE455FNYB5TXC
                    self.through_parts(&path, e.span)?
                };
                member_ref(path, e.span)
            }
            // frob:ticket 01M4W3YJCH948JJQ3HY646D1T7
            Resolved::Part { path } if self.instances.is_instance(&path) => {
                Ok(RefKind::Component(path))
            }
            Resolved::Item {
                path,
                class: ItemClass::Component,
            } => Ok(RefKind::Component(path)),
            Resolved::Variant { enum_path, variant } => Ok(RefKind::Variant { enum_path, variant }),
            // frob:ticket 01M4TRBJ7D2JTGX6XX94BJ1ZGM
            Resolved::Item {
                path,
                class: ItemClass::Table,
            } => Ok(RefKind::Table(path)),
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
            RefKind::Variant { enum_path, variant } => Ok(Val::Variant { enum_path, variant }),
            RefKind::Table(path) => Ok(Val::Table(path)),
            RefKind::Component(path) => Ok(Val::Component(path)),
            RefKind::Member { key, chain } if self.fit_defs.contains_key(&key) => {
                self.fit_member(&key, &chain, e.span)
            }
            RefKind::Member { key, chain } => {
                let role = self.value_of(&key).and_then(|ev| ev.role).map(|r| r.role);
                let v = self.member_value(&key, e.span)?;
                if let [first, rest @ ..] = chain.as_slice()
                    && first == "drawn"
                {
                    let v = self.drawn_member(&key, v, e.span)?;
                    return self.apply_chain(v, role, rest, e.span);
                }
                self.apply_chain(v, role, &chain, e.span)
            }
        }
    }

    /// Reads `NAME.drawn` of a compensated member: the process rule applied to its nominal (language.md 3.8).
    // frob:ticket 01M4Z33NNYYPECFPASHMRQXAZQ
    fn drawn_member(&mut self, key: &CanonPath, designed: Val, span: Span) -> R<Val> {
        let drawn = match (&designed, &self.processes) {
            (Val::Quantity(q), Some(procs)) => procs.drawn_of(key, q),
            _ => None,
        };
        match drawn {
            Some(Ok(q)) => {
                debug!(%key, "evaluator: drawn value read");
                Ok(Val::Quantity(q))
            }
            Some(Err(_)) => {
                trace!(%key, "evaluator: drawn rule failed, reported by Processes::apply");
                Err(Fail::Silent)
            }
            None => Err(diag("EVAL", span, "`.drawn` is not readable on this value")),
        }
    }

    /// Hands the evaluator the compiled processes so checks can read `.drawn`.
    // frob:ticket 01M4Z33NNYYPECFPASHMRQXAZQ
    pub fn set_processes(&mut self, processes: crate::process::Processes) {
        self.processes = Some(Rc::new(processes));
    }

    /// Reads `NAME.clearance` of a `fit` member (language.md 3.7): hole minus shaft, as an interval.
    // frob:ticket 01M4Z33NKSMBJSGKAMVDGYN4CP
    fn fit_member(&mut self, key: &CanonPath, chain: &[String], span: Span) -> R<Val> {
        match chain {
            [name] if name == "clearance" => {}
            _ => {
                return Err(diag(
                    "EVAL",
                    span,
                    "only `.clearance` is readable on a fit here",
                ));
            }
        }
        if let Some(done) = self.fit_done.get(key) {
            return done.clone().map(Val::Quantity).ok_or(Fail::Silent);
        }
        if !self.in_progress.insert(key.clone()) {
            trace!(%key, "evaluator: fit cycle, left to D003");
            return Err(Fail::Silent);
        }
        let out = self.fit_clearance(key);
        self.in_progress.remove(key);
        debug!(%key, ok = out.is_some(), "evaluator: fit clearance evaluated");
        self.fit_done.insert(key.clone(), out.clone());
        out.map(Val::Quantity).ok_or(Fail::Silent)
    }

    /// The clearance of fit `key`, or `None` when an operand or the analysis failed (F001/F002 are the lint's).
    // frob:ticket 01M4Z33NKSMBJSGKAMVDGYN4CP
    fn fit_clearance(&mut self, key: &CanonPath) -> Option<Quant> {
        let FitDef { fit, ctx } = self.fit_defs.get(key)?.clone();
        let operand = |this: &mut Self, p: &ast::Path| -> Option<Toleranced> {
            let Ok(Resolved::Member { path, .. }) = this.resolver.resolve_path(&ctx, p, None)
            else {
                return None;
            };
            match this.value_of(&path)?.value {
                Val::Quantity(q) => crate::fitderive::toleranced(&q).ok(),
                _ => None,
            }
        };
        let (first, second) = match &fit.spec {
            ast::FitSpec::Paths(a, b) => (operand(self, a)?, Some(operand(self, b)?)),
            _ => (operand(self, &fit.on)?, None),
        };
        let result = crate::fits::analyze(&fit.spec, &first, second.as_ref(), None).ok()?;
        let var = self.fresh_var();
        let form = result.clearance.to_affine(var).ok()?;
        Some(Quant {
            form,
            nominal: result.clearance.nominal().clone(),
        })
    }

    /// The value of member `key`; a missing instance member is E002 (absent), a failed one is silent.
    // frob:ticket 01M4W3YJCH948JJQ3HY646D1T7
    fn member_value(&mut self, key: &CanonPath, span: Span) -> R<Val> {
        match self.value_of(key) {
            Some(ev) => Ok(ev.value),
            None if !key.parts.is_empty()
                && (self.absent.contains(key) || !self.defs.contains_key(key)) =>
            {
                Err(diag("E002", span, format!("`{key}` is absent here")))
            }
            None => Err(Fail::Silent),
        }
    }

    /// Reads `chain` from `v`: accessors, component members, row columns, point coordinates.
    // frob:ticket 01M4W3YJCH948JJQ3HY646D1T7
    fn apply_chain(&mut self, v: Val, role: Option<Role>, chain: &[String], span: Span) -> R<Val> {
        let mut v = v;
        for name in chain {
            v = match v {
                Val::Quantity(q) if ACCESSORS.contains(&name.as_str()) => {
                    Val::Quantity(q.accessor(name, role, span)?)
                }
                Val::Component(mut p) => {
                    p.members = vec![MemberSeg::Name(name.clone())];
                    self.member_value(&p, span)?
                }
                Val::Row { table, row } => {
                    let id = ast::Ident {
                        name: name.clone(),
                        span,
                    };
                    self.row_field(&table, row, &id, span)?
                }
                Val::Point(comps) if ["x", "y", "z"].contains(&name.as_str()) => {
                    let i = ["x", "y", "z"].iter().position(|m| m == name).unwrap_or(0);
                    comps
                        .get(i)
                        .cloned()
                        .ok_or_else(|| diag("EVAL", span, format!("this point has no `.{name}`")))?
                }
                _ if ACCESSORS.contains(&name.as_str()) => {
                    return Err(diag("EVAL", span, format!("`.{name}` needs a quantity")));
                }
                _ => {
                    return Err(diag(
                        "EVAL",
                        span,
                        format!("`.{name}` is not readable on this value"),
                    ));
                }
            };
        }
        Ok(v)
    }

    /// Maps a callee-definition path to the current instance's path (`None` outside an instance).
    // frob:ticket 01M4W3YJCH948JJQ3HY646D1T7
    fn instance_path(&self, path: &CanonPath) -> Option<CanonPath> {
        let inst = self.cur_inst.as_ref()?;
        if !path.parts.is_empty() || path.item_only() != inst.callee {
            return None;
        }
        let mut p = inst.prefix.clone();
        p.members = path.members.clone();
        Some(p)
    }

    fn quant(&mut self, sc: &mut Scope, e: &Expr) -> R<Quant> {
        match self.eval(sc, e)? {
            Val::Quantity(q) => Ok(q),
            Val::Inexact(_) => Err(diag(
                "EVAL",
                e.span,
                "an inexact value (sqrt, trigonometry) is not allowed here; round it first with `round_to`",
            )),
            _ => Err(diag("EVAL", e.span, "a quantity is required here")),
        }
    }

    /// Evaluates the base of a `.x` access; a `p in D` re-expression that fails with G003 keeps
    /// that diagnostic, as does any other diagnosed failure (nothing else reads `(p in D).x`); a silent
    /// failure falls through to reference classification.
    // frob:ticket 01M4VQXPDA4TEKE455FNYB5TXC
    fn point_base(&mut self, sc: &mut Scope, base: &Expr) -> R<Option<Val>> {
        let reexpress = matches!(&strip(base).kind, ExprKind::Binary(BinOp::In, ..));
        match self.eval(sc, base) {
            Ok(v) => Ok(Some(v)),
            // frob:ticket 01M4VYZRQKXYE9WEFMDM3797GP
            Err(Fail::Diag(d)) if reexpress => Err(Fail::Diag(d)),
            Err(_) => Ok(None),
        }
    }

    fn eval(&mut self, sc: &mut Scope, e: &Expr) -> R<Val> {
        let span = e.span;
        if let ExprKind::Field(base, member) = &e.kind
            && let Some(i) = ["x", "y", "z"].iter().position(|m| *m == member.name)
            && let Some(Val::Point(comps)) = self.point_base(sc, base)?
        {
            return comps.get(i).cloned().ok_or_else(|| {
                diag(
                    "EVAL",
                    span,
                    format!("this point has no `.{}`", member.name),
                )
            });
        }
        // frob:ticket 01M4TRBJ7D2JTGX6XX94BJ1ZGM
        if let ExprKind::Field(base, member) = &e.kind
            && matches!(
                strip(base).kind,
                ExprKind::Index(..) | ExprKind::Call { .. }
            )
        {
            return match self.eval(sc, base)? {
                Val::Row { table, row } => self.row_field(&table, row, member, span),
                _ => Err(diag(
                    "EVAL",
                    span,
                    format!("`.{}` needs a table row or a point", member.name),
                )),
            };
        }
        if let ExprKind::Field(..) = &e.kind
            && let Some(r) = self.bound_limit(sc, e)
        {
            return r;
        }
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
            ExprKind::Unary(UnOp::Neg, x) => match self.eval(sc, x)? {
                Val::Quantity(q) => Ok(Val::Quantity(q.neg())),
                Val::Inexact(i) => Ok(Val::Inexact(func::inex_neg(&i))),
                _ => Err(diag("EVAL", x.span, "a quantity is required here")),
            },
            ExprKind::Array(xs) => {
                let mut out = Vec::with_capacity(xs.len());
                for x in xs {
                    out.push(self.eval(sc, x)?);
                }
                self.check_homogeneous(&out, span)?;
                Ok(Val::Array(out))
            }
            ExprKind::Match { scrutinee, arms } => self.eval_match(sc, scrutinee, arms, span),
            // frob:ticket 01M4VQXPDA4TEKE455FNYB5TXC
            ExprKind::Index(base, idx) => self.index(sc, base, idx, span),
            ExprKind::Binary(op, l, r) => self.binary(sc, *op, l, r, span),
            ExprKind::Tolerance(inner, terms) => Ok(self.tolerance_val(sc, inner, terms, span)?.0),
            ExprKind::IsoClass(inner, class) => {
                let var = self.fresh_var();
                Ok(Val::Quantity(
                    self.iso_class(sc, inner, class, var, span)?.0,
                ))
            }
            ExprKind::Call {
                callee,
                optional: false,
                args,
            } => self.call(sc, callee, args, span),
            ExprKind::Block(b) => self.eval_block(sc, b),
            // frob:ticket 01M4W3YJCH948JJQ3HY646D1T7
            ExprKind::If {
                cond,
                then,
                otherwise,
            } => match self.eval(sc, cond)? {
                Val::Bool(true) => self.eval_block(sc, then),
                Val::Bool(false) => match otherwise {
                    Some(o) => self.eval(sc, o),
                    None => Err(diag("EVAL", span, "an `if` without `else` has no value")),
                },
                _ => Err(diag("EVAL", cond.span, "an `if` condition must be a Bool")),
            },
            ExprKind::Compare { first, rest } => self.compare_chain(sc, first, rest, span),
            ExprKind::Unary(UnOp::Not, x) => Ok(Val::Bool(!self.boolean(sc, x)?)),
            _ => Err(diag(
                "EVAL",
                span,
                "this construct is not evaluated by this stage",
            )),
        }
    }

    /// Evaluates `e` as a Bool.
    // frob:ticket 01M4W3YJCH948JJQ3HY646D1T7
    fn boolean(&mut self, sc: &mut Scope, e: &Expr) -> R<bool> {
        match self.eval(sc, e)? {
            Val::Bool(b) => Ok(b),
            _ => Err(diag("EVAL", e.span, "a Bool is required here")),
        }
    }

    /// Evaluates a (chained) comparison to a Bool: `==`/`!=` on any like values, ordering on quantities.
    // frob:ticket 01M4W3YJCH948JJQ3HY646D1T7
    fn compare_chain(
        &mut self,
        sc: &mut Scope,
        first: &Expr,
        rest: &[(ast::CmpOp, Expr)],
        span: Span,
    ) -> R<Val> {
        let mut left = self.eval(sc, first)?;
        let mut holds = true;
        for (op, r) in rest {
            let right = self.eval(sc, r)?;
            holds &= compare(*op, &left, &right, span)?;
            left = right;
        }
        Ok(Val::Bool(holds))
    }

    /// Evaluates a block: its `let`s in order, then the tail.
    fn eval_block(&mut self, sc: &mut Scope, b: &Block) -> R<Val> {
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

    fn binary(&mut self, sc: &mut Scope, op: BinOp, l: &Expr, r: &Expr, span: Span) -> R<Val> {
        match op {
            BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div => {
                let (av, bv) = (self.eval(sc, l)?, self.eval(sc, r)?);
                if matches!(av, Val::Point(_) | Val::Array(_))
                    || matches!(bv, Val::Point(_) | Val::Array(_))
                {
                    return point::compound(op, &av, &bv, span);
                }
                let (Val::Quantity(a), Val::Quantity(b)) = (&av, &bv) else {
                    return func::inex_binary(op, &av, &bv, span);
                };
                // Untoleranced operands with different pi exponents sum to an inexact
                // value (Q006, quantities.md 1); only affine forms refuse (5.4).
                let mixes_pi = match op {
                    BinOp::Add => matches!(a.nominal.add(&b.nominal), Sum::Inexact(_)),
                    BinOp::Sub => matches!(a.nominal.sub(&b.nominal), Sum::Inexact(_)),
                    _ => false,
                };
                if mixes_pi && a.form.is_exact() && b.form.is_exact() {
                    debug!(?op, "eval: sum across pi exponents is inexact");
                    return func::inex_binary(op, &av, &bv, span);
                }
                let q = match op {
                    BinOp::Add => a.add(b, span)?,
                    BinOp::Sub => a.sub(b, span)?,
                    BinOp::Mul => a.mul(b, span)?,
                    _ => a.div(b, span)?,
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
            // frob:ticket 01M4VQXPDA4TEKE455FNYB5TXC
            BinOp::In => self.reexpress(sc, l, r, span),
            // frob:ticket 01M4W3YJCH948JJQ3HY646D1T7
            BinOp::And => Ok(Val::Bool(self.boolean(sc, l)? && self.boolean(sc, r)?)),
            BinOp::Or => Ok(Val::Bool(self.boolean(sc, l)? || self.boolean(sc, r)?)),
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
        self.tolerance_on(sc, base, terms, var, span)
    }

    /// Applies a tolerance bracket to an already evaluated base quantity.
    fn tolerance_on(
        &mut self,
        sc: &mut Scope,
        base: Quant,
        terms: &[TolTerm],
        var: VarId,
        span: Span,
    ) -> R<Quant> {
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

    // frob:ticket 01M4VS6WG57MAD0T8GK956NGG9
    /// Evaluates a tolerance suffix; on an array or point it gives every coordinate its own source variable (geometry.md 3).
    fn tolerance_val(
        &mut self,
        sc: &mut Scope,
        inner: &Expr,
        terms: &[TolTerm],
        span: Span,
    ) -> R<(Val, Option<VarId>)> {
        if matches!(inner.kind, ExprKind::Array(_) | ExprKind::Call { .. }) {
            let v = self.eval(sc, inner)?;
            if matches!(v, Val::Array(_) | Val::Point(_)) {
                debug!("evaluator: tolerance on a composite value, one variable per coordinate");
                return Ok((self.tolerate_coords(sc, v, terms, span)?, None));
            }
        }
        let var = self.fresh_var();
        let q = self.tolerance(sc, inner, terms, var, span)?;
        Ok((Val::Quantity(q), Some(var)))
    }

    /// Recurses into arrays and points, tolerancing each quantity leaf independently.
    fn tolerate_coords(&mut self, sc: &mut Scope, v: Val, terms: &[TolTerm], span: Span) -> R<Val> {
        match v {
            Val::Array(items) => items
                .into_iter()
                .map(|i| self.tolerate_coords(sc, i, terms, span))
                .collect::<R<Vec<_>>>()
                .map(Val::Array),
            Val::Point(items) => items
                .into_iter()
                .map(|i| self.tolerate_coords(sc, i, terms, span))
                .collect::<R<Vec<_>>>()
                .map(Val::Point),
            Val::Quantity(q) => {
                let var = self.fresh_var();
                Ok(Val::Quantity(self.tolerance_on(sc, q, terms, var, span)?))
            }
            other => Ok(other),
        }
    }

    /// Applies an ISO 286 class to a nominal: the toleranced quantity and the role its letter gives
    /// (quantities.md 5.1, 5.6). F001 and F002 come from the table lookup.
    // frob:ticket 01M4V1HDVGKT6AVWCWY7078TNV
    fn iso_class(
        &mut self,
        sc: &mut Scope,
        inner: &Expr,
        class: &ast::Ident,
        var: VarId,
        span: Span,
    ) -> R<(Quant, RoleInfo)> {
        let base = self.quant(sc, inner)?;
        let (side, _) =
            crate::fits::class_side(base.dim(), &base.nominal, &class.name).map_err(|e| {
                let msg = crate::fits::failure_message(&e);
                diag(e.code().unwrap_or("EVAL"), span, msg)
            })?;
        let form = side.value.to_affine(var).map_err(|e| tol_diag(&e, span))?;
        trace!(class = %class.name, ?var, "evaluator: ISO class owns a source variable");
        Ok((
            Quant {
                form,
                nominal: base.nominal,
            },
            RoleInfo {
                role: side.role.role,
                source: RoleSource::IsoClass,
            },
        ))
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
        role: None,
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

/// Compares two values: equality on any like values, ordering on same-dimension quantities.
fn compare(op: ast::CmpOp, a: &Val, b: &Val, span: Span) -> R<bool> {
    use ast::CmpOp::{Eq, Ge, Gt, Le, Lt, Ne};
    let ord = match (a, b) {
        (Val::Quantity(x), Val::Quantity(y)) => {
            x.dim()
                .check_same_dimension(y.dim())
                .map_err(|e| diag("Q002", span, e.to_string()))?;
            cmp_mag(&x.nominal, &y.nominal)
        }
        _ => {
            return match op {
                Eq => Ok(a == b),
                Ne => Ok(a != b),
                _ => Err(diag("EVAL", span, "ordering needs two quantities")),
            };
        }
    };
    Ok(match op {
        Eq => ord.is_eq(),
        Ne => ord.is_ne(),
        Lt => ord.is_lt(),
        Le => ord.is_le(),
        Gt => ord.is_gt(),
        Ge => ord.is_ge(),
    })
}

/// The path truncated to its first member (the definition key).
fn first_member(path: &CanonPath) -> CanonPath {
    let mut k = path.clone();
    k.members.truncate(1);
    k
}

/// Splits a member path into its definition key and the names read after it.
fn member_ref(path: CanonPath, span: Span) -> R<RefKind> {
    let mut chain = Vec::new();
    for m in path.members.iter().skip(1) {
        match m {
            MemberSeg::Name(n) => chain.push(n.clone()),
            MemberSeg::Index(_) => {
                return Err(diag("EVAL", span, "member chain is not evaluable here"));
            }
        }
    }
    Ok(RefKind::Member {
        key: first_member(&path),
        chain,
    })
}

/// Indexes the `fit` members of a component without parameters.
// frob:ticket 01M4Z33NKSMBJSGKAMVDGYN4CP
fn index_fits<'p>(
    out: &mut BTreeMap<CanonPath, FitDef<'p>>,
    package: &str,
    module: &[String],
    c: &'p Component,
) {
    if !c.params.is_empty() {
        return;
    }
    let ctx = Ctx {
        package: package.to_owned(),
        module: module.to_vec(),
        component: Some(c.name.name.clone()),
        locals: Vec::new(),
    };
    for m in &c.members {
        if let MemberKind::Fit(f) = &m.kind {
            let mut key = CanonPath::item(package, module, &c.name.name);
            key.members.push(MemberSeg::Name(f.name.name.clone()));
            trace!(%key, "evaluator: fit indexed");
            out.insert(
                key,
                FitDef {
                    fit: f,
                    ctx: ctx.clone(),
                },
            );
        }
    }
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
        let (name, expr, is_let, span, ty) = match &m.kind {
            MemberKind::Param(p) => (&p.name.name, &p.value, false, p.name.span, p.ty.as_ref()),
            MemberKind::Let(l) => (&l.name.name, &l.value, true, l.span, l.ty.as_ref()),
            _ => continue,
        };
        let mut key = item.clone();
        key.members.push(MemberSeg::Name(name.clone()));
        defs.insert(
            key,
            Def {
                expr,
                ty,
                is_let,
                ctx: ctx.clone(),
                span,
                inst: None,
                fixed: None,
                when: None,
                role: crate::attrs::explicit_role(&m.attrs, &m.trailing),
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
    use cadetta_units::RootOp;

    pub(super) fn file(src: &str) -> File {
        let p = parse(src);
        assert!(p.diags.is_empty(), "{:?}", p.diags);
        p.file
    }

    pub(super) fn run(f: &File) -> Evaluation {
        let pkgs = vec![PackageSrc {
            defaults: Default::default(),
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

    fn val(ev: &Evaluation, comp: &str, member: &str) -> Val {
        ev.get(&at(comp, member)).expect("evaluated").value.clone()
    }

    fn has(ev: &Evaluation, code: &str, needle: &str) -> bool {
        ev.diagnostics
            .iter()
            .any(|d| d.code == code && d.message.contains(needle))
    }

    // frob:tests user_function_with_lets_and_defaults
    #[test]
    fn user_function_with_lets_and_defaults() {
        let f = file(
            "fn area(w: Length, h: Length = 2mm) -> Area { let a = w * h; a }\n\
             model M {\n  a = area(3mm)\n  b = area(3mm, h = 4mm)\n  c = area(h = 5mm, w = 2mm)\n}\n",
        );
        let ev = run(&f);
        assert!(ev.diagnostics.is_empty(), "{:?}", ev.diagnostics);
        let sq = |n: i64| m(n, 1_000_000);
        assert_eq!(quant(&ev, "M", "a").nominal, sq(6));
        assert_eq!(quant(&ev, "M", "b").nominal, sq(12));
        assert_eq!(quant(&ev, "M", "c").nominal, sq(10));
    }

    // frob:tests user_function_argument_errors_are_e006
    #[test]
    fn user_function_argument_errors_are_e006() {
        let f = file(
            "fn id(w: Length) -> Length { w }\n\
             model M {\n  a = id()\n  b = id(1mm, 2mm)\n  c = id(q = 1mm)\n}\n",
        );
        let ev = run(&f);
        assert!(has(&ev, "E006", "missing argument"), "{:?}", ev.diagnostics);
        assert!(has(&ev, "E006", "takes 1"), "{:?}", ev.diagnostics);
        assert!(has(&ev, "E006", "no parameter"), "{:?}", ev.diagnostics);
    }

    // frob:tests recursion_is_rejected
    #[test]
    fn recursion_is_rejected() {
        let f = file(
            "fn f(n: Count) -> Count { f(n) }\n\
             fn a(n: Count) -> Count { b(n) }\n\
             fn b(n: Count) -> Count { a(n) }\n\
             fn ok(n: Count) -> Count { n }\n\
             model M {\n  x = f(1)\n  y = ok(2)\n}\n",
        );
        let ev = run(&f);
        let rec: Vec<_> = ev
            .diagnostics
            .iter()
            .filter(|d| d.message.contains("recursive"))
            .collect();
        assert_eq!(rec.len(), 3, "{:?}", ev.diagnostics);
        assert!(ev.get(&at("M", "x")).is_none());
        assert!(ev.get(&at("M", "y")).is_some());
    }

    // frob:tests match_non_exhaustive_names_missing_variants
    #[test]
    fn match_non_exhaustive_names_missing_variants() {
        let f = file(
            "enum Mat { Al, Petg, Steel }\n\
             model M {\n  m = Mat::Al\n  t = match m { Mat::Petg => 2.4mm }\n}\n",
        );
        let ev = run(&f);
        assert!(has(&ev, "E009", "`Mat::Al`"), "{:?}", ev.diagnostics);
        assert!(has(&ev, "E009", "`Mat::Steel`"), "{:?}", ev.diagnostics);
        assert!(!has(&ev, "E009", "`Mat::Petg`"));
    }

    // frob:tests match_exhaustive_and_wildcard_select_an_arm
    #[test]
    fn match_exhaustive_and_wildcard_select_an_arm() {
        let f = file(
            "enum Mat { Al, Petg }\n\
             fn thick(m: Mat) -> Length { match m { Al => 1.6mm, Petg => 2.4mm } }\n\
             model M {\n  m = Mat::Petg\n  a = match m { Mat::Al => 1mm, _ => 2mm }\n  \
             b = thick(Petg)\n  c = thick(Mat::Al)\n}\n",
        );
        let ev = run(&f);
        assert!(ev.diagnostics.is_empty(), "{:?}", ev.diagnostics);
        assert_eq!(quant(&ev, "M", "a").nominal, m(2, 1000));
        assert_eq!(quant(&ev, "M", "b").nominal, m(24, 10_000));
        assert_eq!(quant(&ev, "M", "c").nominal, m(16, 10_000));
    }

    // frob:tests min_max_abs_follow_intervals
    #[test]
    fn min_max_abs_follow_intervals() {
        let f = file(
            "model M {\n  a = min(3mm, 2mm, 5mm)\n  b = max(3mm, 2mm)\n  \
             c = abs(-4mm)\n  t = 10mm [+-1mm]\n  let d = max(t, 10.5mm)\n  let e = min(t, 20mm)\n  \
             z = 0mm [+-1mm]\n  let f = abs(z)\n  g = min(1mm, 1s)\n}\n",
        );
        let ev = run(&f);
        assert_eq!(quant(&ev, "M", "a").nominal, m(2, 1000));
        assert_eq!(quant(&ev, "M", "b").nominal, m(3, 1000));
        assert_eq!(quant(&ev, "M", "c").nominal, m(4, 1000));
        // t in [9, 11] mm vs 10.5mm: overlap, hull [10.5, 11].
        let d = quant(&ev, "M", "d");
        assert_eq!(d.form.bounds().unwrap(), (m(105, 10_000), m(11, 1000)));
        // t is certainly below 20mm: returned unchanged, variable kept.
        assert_eq!(quant(&ev, "M", "e").form, quant(&ev, "M", "t").form);
        let f = quant(&ev, "M", "f");
        assert_eq!(f.form.bounds().unwrap(), (m(0, 1), m(1, 1000)));
        assert!(has(&ev, "Q002", ""), "{:?}", ev.diagnostics);
    }

    // frob:tests sqrt_exact_inexact_and_dimension
    #[test]
    fn sqrt_exact_inexact_and_dimension() {
        let f = file(
            "model M {\n  a = sqrt(16mm^2)\n  b = sqrt(2mm^2)\n  c = round_to(sqrt(2mm^2), 0.001mm)\n  \
             d = sqrt(4mm)\n  e = sqrt(-1mm^2)\n  let g = sqrt(9mm^2 [+-1mm^2])\n}\n",
        );
        let ev = run(&f);
        assert_eq!(quant(&ev, "M", "a").nominal, m(4, 1000));
        assert!(a_is_exact(&ev));
        match val(&ev, "M", "b") {
            Val::Inexact(i) => {
                assert_eq!(i.root, RootOp::Sqrt);
                assert!(i.lo <= 2f64.sqrt() * 1e-3 && 2f64.sqrt() * 1e-3 <= i.hi);
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(quant(&ev, "M", "c").nominal, m(1414, 1_000_000));
        assert!(has(&ev, "Q011", ""), "{:?}", ev.diagnostics);
        assert!(has(&ev, "EVAL", "negative") || has(&ev, "EVAL", "sqrt"));
        assert!(matches!(val(&ev, "M", "g"), Val::Inexact(_)));
    }

    fn a_is_exact(ev: &Evaluation) -> bool {
        quant(ev, "M", "a").form.is_exact()
    }

    // frob:tests trig_exact_special_angles_and_inexact
    #[test]
    fn trig_exact_special_angles_and_inexact() {
        let f = file(
            "model M {\n  a = sin(90deg)\n  b = cos(60deg)\n  c = tan(45deg)\n  \
             d = sin(1rad)\n  e = tan(90deg)\n  g = sin(2mm)\n  h = atan2(1mm, 1mm)\n  \
             i = atan2(1mm, 2mm)\n  j = atan2(1mm, 1s)\n  k = round_to(d, 0.01)\n}\n",
        );
        let ev = run(&f);
        assert_eq!(quant(&ev, "M", "a").nominal, m(1, 1));
        assert_eq!(quant(&ev, "M", "b").nominal, m(1, 2));
        assert_eq!(quant(&ev, "M", "c").nominal, m(1, 1));
        match val(&ev, "M", "d") {
            Val::Inexact(i) => {
                assert_eq!(i.root, RootOp::Sin);
                assert!((i.nominal - 1f64.sin()).abs() < 1e-12);
            }
            other => panic!("{other:?}"),
        }
        assert!(ev.get(&at("M", "e")).is_none());
        assert!(has(&ev, "Q002", "angle"), "{:?}", ev.diagnostics);
        let h = quant(&ev, "M", "h");
        assert_eq!(h.nominal.pi_exp(), 1);
        assert!(matches!(val(&ev, "M", "i"), Val::Inexact(_)));
        assert!(has(&ev, "Q002", ""), "{:?}", ev.diagnostics);
        assert_eq!(quant(&ev, "M", "k").nominal, m(84, 100));
    }

    // frob:tests rounding_to_count_and_count_of_array
    #[test]
    fn rounding_to_count_and_count_of_array() {
        let f = file(
            "model M {\n  a = floor(2.7)\n  b = ceil(2.1)\n  c = round(2.5)\n  d = round(-2.5)\n  \
             e = count([1mm, 2mm, 3mm])\n  g = floor(2mm)\n  h = round(sqrt(2))\n}\n",
        );
        let ev = run(&f);
        assert_eq!(quant(&ev, "M", "a").nominal, m(2, 1));
        assert_eq!(quant(&ev, "M", "b").nominal, m(3, 1));
        assert_eq!(quant(&ev, "M", "c").nominal, m(3, 1));
        assert_eq!(quant(&ev, "M", "d").nominal, m(-3, 1));
        assert_eq!(quant(&ev, "M", "e").nominal, m(3, 1));
        assert!(has(&ev, "Q002", ""), "{:?}", ev.diagnostics);
        assert_eq!(quant(&ev, "M", "h").nominal, m(1, 1));
    }

    // frob:tests round_to_inexact_near_tie_is_q010
    #[test]
    fn round_to_inexact_near_tie_is_q010() {
        let f = file(
            "model M {\n  a = round_to(sqrt(0.25mm^2) * 1, 0.001mm)\n  b = round_to(sqrt(2) * 1, 1)\n}\n",
        );
        let ev = run(&f);
        assert_eq!(quant(&ev, "M", "a").nominal, m(5, 10_000));
        assert_eq!(quant(&ev, "M", "b").nominal, m(1, 1));
    }

    // frob:tests fmt_renders_by_display_rules
    #[test]
    fn fmt_renders_by_display_rules() {
        let f = file(
            "enum Thread { M5, M6 }\n\
             model M {\n  thread = Thread::M6\n  length = 12.5mm [+-0.1mm]\n  \
             a = fmt(\"{} x {}\", thread, length)\n  b = fmt(\"{{}} {}\", 2)\n  \
             c = fmt(\"{}\", 3in)\n  d = fmt(\"{} {}\", 1)\n  e = fmt(\"{}\", 2s)\n}\n",
        );
        let ev = run(&f);
        assert_eq!(val(&ev, "M", "a"), Val::Text("M6 x 12.5mm".to_owned()));
        assert_eq!(val(&ev, "M", "b"), Val::Text("{} 2".to_owned()));
        assert_eq!(val(&ev, "M", "c"), Val::Text("3in".to_owned()));
        assert!(has(&ev, "E006", "placeholder"), "{:?}", ev.diagnostics);
        assert_eq!(val(&ev, "M", "e"), Val::Text("2s".to_owned()));
    }

    const TABLES: &str = "enum Th { A, B }\n\
        table T { key t: Th\n cols major: Length, fine: Ratio\n \
        Th::A => 5mm, 0.5\n Th::B => 6mm, 0.25 }\n\
        table D { key d: Length\n 3mm\n 4mm\n 5mm }\n";

    // frob:tests table_index_with_a_member_key_selects_the_row
    #[test]
    fn table_index_with_a_member_key_selects_the_row() {
        let src = format!(
            "{TABLES}model M {{\n  th = Th::B\n  major = T[th].major\n  fine = T[Th::A].fine\n  \
             miss = T[Th::A].nope\n}}\n"
        );
        let f = file(&src);
        let ev = run(&f);
        assert_eq!(quant(&ev, "M", "major").nominal, m(6, 1000));
        assert_eq!(quant(&ev, "M", "fine").nominal, m(1, 2));
        assert!(has(&ev, "E008", "nope"), "{:?}", ev.diagnostics);
    }

    // frob:tests table_index_miss_is_e007
    #[test]
    fn table_index_miss_is_e007() {
        let src = format!("{TABLES}model M {{\n  x = D[3.5mm].key\n}}\n");
        let f = file(&src);
        let ev = run(&f);
        assert!(has(&ev, "E007", "no row"), "{:?}", ev.diagnostics);
    }

    // frob:tests range_lookups_and_table_by_name_values
    #[test]
    fn range_lookups_and_table_by_name_values() {
        let src = format!(
            "{TABLES}model M {{\n  want = 3.2mm\n  drills = D\n  \
             up = D.at_least(want).key\n  down = D.at_most(want).key\n  \
             near = D.nearest(4.4mm).key\n  via = drills.at_least(4.1mm).key\n}}\n"
        );
        let f = file(&src);
        let ev = run(&f);
        assert!(ev.diagnostics.is_empty(), "{:?}", ev.diagnostics);
        assert_eq!(quant(&ev, "M", "up").nominal, m(4, 1000));
        assert_eq!(quant(&ev, "M", "down").nominal, m(3, 1000));
        assert_eq!(quant(&ev, "M", "near").nominal, m(4, 1000));
        assert_eq!(quant(&ev, "M", "via").nominal, m(5, 1000));
        assert!(matches!(val(&ev, "M", "drills"), Val::Table(_)));
    }

    fn codes(ev: &Evaluation) -> Vec<&'static str> {
        ev.diagnostics.iter().map(|d| d.code).collect()
    }

    // frob:tests iso_class_applies_table_deviations_and_hole_role
    #[test]
    fn iso_class_applies_table_deviations_and_hole_role() {
        let ev = run(&file(
            "model P {\n  a = 20mm H7\n  hi = a.max\n  lo = a.min\n  m = a.mmc\n}\n",
        ));
        assert!(ev.diagnostics.is_empty(), "{:?}", ev.diagnostics);
        assert_eq!(quant(&ev, "P", "hi").nominal, m(20_021, 1_000_000));
        assert_eq!(quant(&ev, "P", "lo").nominal, m(20, 1_000));
        let role = ev.get(&at("P", "a")).expect("a").role.expect("role");
        assert_eq!(role.role, Role::Hole);
        assert_eq!(role.source, RoleSource::IsoClass);
        assert_eq!(quant(&ev, "P", "m").nominal, m(20, 1_000));
    }

    // frob:tests iso_class_lowercase_letter_is_shaft
    #[test]
    fn iso_class_lowercase_letter_is_shaft() {
        let ev = run(&file("model P {\n  a = 20mm g6\n  m = a.mmc\n}\n"));
        assert!(ev.diagnostics.is_empty(), "{:?}", ev.diagnostics);
        let role = ev.get(&at("P", "a")).expect("a").role.expect("role");
        assert_eq!(role.role, Role::Shaft);
        // g6 at 20mm: upper deviation -7um, so the largest size is 19.993mm.
        assert_eq!(quant(&ev, "P", "m").nominal, m(19_993, 1_000_000));
    }

    // frob:tests iso_class_out_of_range_is_f002
    #[test]
    fn iso_class_out_of_range_is_f002() {
        let ev = run(&file("model P {\n  a = 20000mm H7\n}\n"));
        assert_eq!(codes(&ev), vec!["F002"]);
        assert!(ev.get(&at("P", "a")).is_none());
    }

    // frob:tests mmc_without_role_is_e010
    #[test]
    fn mmc_without_role_is_e010() {
        let ev = run(&file(
            "model P {\n  a = 5mm [+-0.1mm]\n  b = a.mmc\n  c = a.lmc\n}\n",
        ));
        assert_eq!(codes(&ev), vec!["E010", "E010"]);
    }

    // frob:tests fit_side_role_resolves_mmc_lmc
    #[test]
    fn fit_side_role_resolves_mmc_lmc() {
        let ev = run(&file(
            "model P {\n  bore = 10mm [+-0.1mm]\n  pin = 10mm [+-0.1mm]\n  \
             lone = 10mm [+-0.1mm]\n  fit seat = bore / pin\n  fit snug = p6 on lone\n  \
             bm = bore.mmc\n  bl = bore.lmc\n  pm = pin.mmc\n  pl = pin.lmc\n  \
             lm = lone.mmc\n}\n",
        ));
        assert!(ev.diagnostics.is_empty(), "{:?}", ev.diagnostics);
        let bore = ev.get(&at("P", "bore")).expect("bore").role.expect("role");
        assert_eq!((bore.role, bore.source), (Role::Hole, RoleSource::FitSide));
        assert_eq!(quant(&ev, "P", "bm").nominal, m(99, 10_000));
        assert_eq!(quant(&ev, "P", "bl").nominal, m(101, 10_000));
        assert_eq!(quant(&ev, "P", "pm").nominal, m(101, 10_000));
        assert_eq!(quant(&ev, "P", "pl").nominal, m(99, 10_000));
        // `p6 on lone` makes `lone` the hole (the mate of the shaft class).
        assert_eq!(quant(&ev, "P", "lm").nominal, m(99, 10_000));
    }

    // frob:tests fit_side_role_yields_to_explicit_role
    #[test]
    fn fit_side_role_yields_to_explicit_role() {
        let ev = run(&file(
            "model P {\n  #[role(shaft)]\n  a = 10mm [+-0.1mm]\n  b = 10mm [+-0.1mm]\n  \
             fit f = a / b\n  am = a.mmc\n}\n",
        ));
        assert!(ev.diagnostics.is_empty(), "{:?}", ev.diagnostics);
        let a = ev.get(&at("P", "a")).expect("a").role.expect("role");
        assert_eq!(a.source, RoleSource::Attribute);
        assert_eq!(quant(&ev, "P", "am").nominal, m(101, 10_000));
    }

    // frob:tests role_attribute_picks_material_side
    #[test]
    fn role_attribute_picks_material_side() {
        let ev = run(&file(
            "model P {\n  #[role(hole)]\n  a = 10mm [+-0.1mm]\n  m = a.mmc\n  l = a.lmc\n  \
             #[role(shaft)]\n  s = 10mm [+-0.1mm]\n  sm = s.mmc\n  sl = s.lmc\n  r = a\n  rm = r.mmc\n}\n",
        ));
        assert!(ev.diagnostics.is_empty(), "{:?}", ev.diagnostics);
        assert_eq!(quant(&ev, "P", "m").nominal, m(99, 10_000));
        assert_eq!(quant(&ev, "P", "l").nominal, m(101, 10_000));
        assert_eq!(quant(&ev, "P", "sm").nominal, m(101, 10_000));
        assert_eq!(quant(&ev, "P", "sl").nominal, m(99, 10_000));
        let r = ev.get(&at("P", "r")).expect("r").role.expect("inherited");
        assert_eq!(r.source, RoleSource::Inherited);
        assert_eq!(quant(&ev, "P", "rm").nominal, m(99, 10_000));
    }

    // frob:tests fit_clearance_member_is_readable
    #[test]
    fn fit_clearance_member_is_readable() {
        let ev = run(&file(
            "model P {\n  bore = 20mm\n  fit seat = H7/p6 on bore\n  let c = seat.clearance\n  \
             hi = c.max\n  lo = c.min\n}\n",
        ));
        assert!(ev.diagnostics.is_empty(), "{:?}", ev.diagnostics);
        // H7 spans 0..+21um and p6 +22..+35um: clearance is -35um .. -1um.
        assert_eq!(quant(&ev, "P", "hi").nominal, m(-1, 1_000_000));
        assert_eq!(quant(&ev, "P", "lo").nominal, m(-35, 1_000_000));
    }

    // frob:tests drawn_member_reads_rule_applied_to_nominal
    #[test]
    fn drawn_member_reads_rule_applied_to_nominal() {
        let f = file(
            "process Fdm {\n  hole_comp = 0.2mm\n  rule hole(d: Length) -> Length = d + hole_comp\n}\n\
             model Case {\n  #![process(Fdm)]\n  insert_hole = 3.4mm  #[hole]\n  \
             let d = insert_hole.drawn\n  let m = insert_hole\n  let bad = m.drawn\n}\n",
        );
        let pkgs = vec![PackageSrc {
            defaults: Default::default(),
            name: "p".to_owned(),
            deps: Vec::new(),
            modules: vec![ModuleSrc {
                path: Vec::new(),
                file: &f,
                enums: &[],
            }],
        }];
        let (resolver, errs) = Resolver::new(pkgs.clone());
        assert!(errs.is_empty());
        let units = UnitTable::builtin();
        let mut e = Evaluator::new(&resolver, &units, &pkgs);
        let procs = crate::process::Processes::compile(&mut e, &pkgs);
        e.set_processes(procs);
        let ev = e.evaluate();
        // The drawn value is 3.4mm + 0.2mm = 3.6mm; the plain read stays the designed 3.4mm.
        assert_eq!(quant(&ev, "Case", "d").nominal, m(36, 10_000));
        assert_eq!(quant(&ev, "Case", "m").nominal, m(34, 10_000));
        // `.drawn` on a member with no role is still not readable.
        assert!(
            ev.diagnostics
                .iter()
                .any(|d| d.message.contains("`.drawn` is not readable")),
            "{:?}",
            ev.diagnostics
        );
    }
}
