//! frob:ticket 01M4STAN8QG3QWZQCH5XN3QHMT
//! `cadetta solve [PATH-GLOB...] [--module PATH]`: run stale or missing solvers
//! and store their results beside the script (solvers.md 4).

use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;

use cadetta_sema::eval::{Evaluator, Quant, Selection};
use cadetta_sema::modules::module_path_of;
use cadetta_sema::resolve::{CanonPath, Ctx, MemberSeg, ModuleSrc, PackageSrc, Resolver};
use cadetta_solve::decl::{SolverDecl, lower_solver};
use cadetta_solve::protocol::{InputValue, Request};
use cadetta_solve::results::{
    CurrentDigests, Lookup, ResultFile, SolveResult, digest_bytes, input_digest, out_path,
};
use cadetta_solve::transport::{self, SolverConfig, TransportError};
use cadetta_solve::variants::{self, VariantRequest};
use cadetta_syntax::ast::{Bind, File, ItemKind, MemberKind, Solver, SolverMemberKind};
use cadetta_syntax::parser::parse;
use cadetta_units::UnitTable;
use cadetta_units::display::{DisplaySpec, format_value};
use cadetta_units::kind::KindTable;

use crate::config::Transport as CfgTransport;
use crate::project::{Module, PackageDir, Project};
use crate::{EXIT_OK, EXIT_USAGE, Format, Globals};

/// Exit code when a solver run or declaration fails (config-cli.md 3).
pub const EXIT_FAILED: u8 = 1;

/// Schema tag of the JSON report; the schema is `docs/schemas/solve-report.json`.
pub const REPORT_SCHEMA: &str = "cadetta.solve-report/1";

const USAGE: &str = "usage: cadetta solve [PATH-GLOB...] [--module PATH] [--variants]";

/// The parsed command line of `cadetta solve`.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct SolveArgs {
    /// Solver path globs (`*` and `?`); empty selects every solver.
    pub globs: Vec<String>,
    /// Restrict to the solvers declared in this module file (relative to the package).
    pub module: Option<String>,
    /// Also solve each approved alternate whose inputs differ (solvers.md 4.1).
    pub variants: bool,
}

/// Parse the words after `solve`; the error is a usage message.
pub fn parse_args(args: &[String]) -> Result<SolveArgs, String> {
    let mut parsed = SolveArgs::default();
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--module" => match it.next() {
                Some(path) => parsed.module = Some(path.clone()),
                None => return Err("`--module` needs a path".to_owned()),
            },
            "--variants" => parsed.variants = true,
            flag if flag.starts_with('-') => return Err(format!("unknown flag `{flag}`")),
            glob => parsed.globs.push(glob.to_owned()),
        }
    }
    Ok(parsed)
}

/// Match `text` against `pattern` where `*` is any run and `?` any one character.
pub fn glob_match(pattern: &str, text: &str) -> bool {
    let (p, t): (Vec<char>, Vec<char>) = (pattern.chars().collect(), text.chars().collect());
    let (mut pi, mut ti, mut star, mut mark) = (0, 0, None, 0);
    while ti < t.len() {
        if pi < p.len() && (p[pi] == '?' || p[pi] == t[ti]) {
            pi += 1;
            ti += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star = Some(pi);
            mark = ti;
            pi += 1;
        } else if let Some(s) = star {
            pi = s + 1;
            mark += 1;
            ti = mark;
        } else {
            return false;
        }
    }
    p[pi..].iter().all(|&c| c == '*')
}

/// What happened to one solver in a `solve` run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    /// The solver ran and its result was stored.
    Solved,
    /// The stored result was current, so nothing ran.
    Fresh,
    /// The declaration, inputs or run failed.
    Failed,
}

/// One row of the JSON report.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Entry {
    /// Canonical solver path.
    pub solver: String,
    /// Outcome for this solver.
    pub status: Status,
    /// Approved alternates (variants) solved alongside; empty for the primary row.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub variants: Vec<String>,
    /// Diagnostic code when `status` is `failed`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    /// Failure message when `status` is `failed`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl Entry {
    fn new(solver: &str, status: Status) -> Self {
        Entry {
            solver: solver.to_owned(),
            status,
            variants: Vec::new(),
            code: None,
            message: None,
        }
    }
}

/// The JSON report of a whole run.
#[derive(Debug, serde::Serialize)]
struct Report<'a> {
    schema: &'static str,
    ok: bool,
    solvers: &'a [Entry],
}

/// One module parsed and ready to resolve.
struct Parsed<'m> {
    module: &'m Module,
    path: Vec<String>,
    file: File,
}

/// One solver found in source, with the context its inputs evaluate in.
struct Target<'a> {
    path: String,
    ast: &'a Solver,
    ctx: Ctx,
    package_dir: &'a Path,
}

fn package_name(pkg: &PackageDir) -> String {
    pkg.config
        .package
        .as_ref()
        .and_then(|p| p.name.clone())
        .unwrap_or_else(|| "unnamed".to_owned())
}

/// Build the resolver's package sources; enums are indexed by the resolver from the AST itself.
fn package_sources<'a>(
    project: &'a Project,
    names: &[String],
    all: &'a [Vec<Parsed<'_>>],
) -> Vec<PackageSrc<'a>> {
    project
        .packages
        .iter()
        .zip(names)
        .zip(all)
        .map(|((pkg, name), parsed)| PackageSrc {
            name: name.clone(),
            deps: pkg.config.dependencies.keys().cloned().collect(),
            modules: parsed
                .iter()
                .map(|m| ModuleSrc {
                    path: m.path.clone(),
                    file: &m.file,
                    enums: &[],
                })
                .collect(),
        })
        .collect()
}

fn parse_package<'m>(pkg: &'m PackageDir, err: &mut dyn Write) -> Option<Vec<Parsed<'m>>> {
    let mut parsed = Vec::new();
    let mut ok = true;
    for module in &pkg.modules {
        let src_rel = module.rel_path.as_str();
        let path = match module_path_of(src_rel) {
            Ok(p) => p,
            Err(error) => {
                tracing::error!(file = %module.file.display(), %error, "solve: bad module path");
                let _ = writeln!(err, "error: {}: {error}", module.file.display());
                ok = false;
                continue;
            }
        };
        let result = parse(&module.source);
        if !result.diags.is_empty() {
            for d in &result.diags {
                tracing::error!(file = %module.file.display(), code = ?d.code, "solve: syntax error");
                let _ = writeln!(err, "error: {}: {}", module.file.display(), d.message);
            }
            ok = false;
            continue;
        }
        parsed.push(Parsed {
            module,
            path,
            file: result.file,
        });
    }
    ok.then_some(parsed)
}

fn targets<'a>(
    pkg: &'a PackageDir,
    name: &str,
    parsed: &'a [Parsed<'_>],
    args: &SolveArgs,
) -> Vec<Target<'a>> {
    let mut found = Vec::new();
    for m in parsed {
        if args.module.as_deref().is_some_and(|want| {
            want.trim_start_matches("./") != m.module.rel_path
                && !m.module.file.ends_with(want.trim_start_matches("./"))
        }) {
            tracing::debug!(module = %m.module.rel_path, "solve: module filtered out");
            continue;
        }
        let ctx = |component: Option<String>| Ctx {
            package: name.to_owned(),
            module: m.path.clone(),
            component,
            locals: Vec::new(),
        };
        for item in m.file.items() {
            match &item.kind {
                ItemKind::Solver(s) => found.push(Target {
                    path: CanonPath::item(name, &m.path, &s.name.name).to_string(),
                    ast: s,
                    ctx: ctx(None),
                    package_dir: &pkg.dir,
                }),
                ItemKind::Component(c) => {
                    for member in &c.members {
                        if let MemberKind::Solver(s) = &member.kind {
                            let mut p = CanonPath::item(name, &m.path, &c.name.name);
                            p.members.push(MemberSeg::Name(s.name.name.clone()));
                            found.push(Target {
                                path: p.to_string(),
                                ast: s,
                                ctx: ctx(Some(c.name.name.clone())),
                                package_dir: &pkg.dir,
                            });
                        }
                    }
                }
                _ => {}
            }
        }
    }
    found.retain(|t| {
        let keep = args.globs.is_empty() || args.globs.iter().any(|g| glob_match(g, &t.path));
        if !keep {
            tracing::debug!(solver = %t.path, "solve: filtered out by glob");
        }
        keep
    });
    found
}

/// The unit string for a dimension's SI exponents, such as `kg*m/s^2`.
fn si_unit(dim: &cadetta_units::Dim) -> String {
    const NAMES: [&str; 7] = ["m", "kg", "s", "A", "K", "mol", "cd"];
    let term = |name: &str, e: i32| {
        if e.abs() == 1 {
            name.to_owned()
        } else {
            format!("{name}^{}", e.abs())
        }
    };
    let (mut num, mut den) = (Vec::new(), Vec::new());
    for (name, &e) in NAMES.iter().zip(dim.dimension.si()) {
        match e {
            0 => {}
            e if e > 0 => num.push(term(name, e)),
            e => den.push(term(name, e)),
        }
    }
    let top = if num.is_empty() {
        "1".to_owned()
    } else {
        num.join("*")
    };
    match den.len() {
        0 => top,
        1 => format!("{top}/{}", den[0]),
        _ => format!("{top}/({})", den.join("*")),
    }
}

/// Lower an evaluated quantity to a request input (solvers.md 3).
fn input_value(table: &UnitTable, q: &Quant) -> InputValue {
    let si = q.nominal.to_f64();
    let unit = si_unit(q.dim());
    let lit = format_value(table, &q.nominal, q.dim(), &DisplaySpec::Unit(unit.clone()))
        .unwrap_or_else(|error| {
            tracing::debug!(%error, %unit, "solve: literal falls back to the float text");
            format!("{si}")
        });
    let (min, max) = match q.form.bounds() {
        Ok((lo, hi)) if !q.form.is_exact() => (Some(lo.to_f64()), Some(hi.to_f64())),
        _ => (None, None),
    };
    InputValue { lit, si, min, max }
}

fn config_map(project: &Project) -> BTreeMap<String, SolverConfig> {
    project
        .config
        .solvers
        .iter()
        .map(|(name, s)| {
            let transport = match s.transport {
                CfgTransport::Stdio => transport::Transport::Stdio,
                CfgTransport::File => transport::Transport::File,
            };
            (
                name.clone(),
                SolverConfig {
                    command: s.command.clone(),
                    transport,
                    timeout: s.timeout,
                },
            )
        })
        .collect()
}

fn read_digest(path: &Path) -> Result<String, std::io::Error> {
    std::fs::read(path).map(|bytes| digest_bytes(&bytes))
}

/// Run `cadetta solve`; `args` are the words after `solve`. Returns the exit code.
pub fn run_solve(
    args: &[String],
    globals: &Globals,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8 {
    let args = match parse_args(args) {
        Ok(a) => a,
        Err(message) => {
            tracing::error!(%message, "solve: bad arguments");
            let _ = writeln!(err, "error: {message}; {USAGE}");
            return EXIT_USAGE;
        }
    };
    let (project, scope) = match Project::open(globals.cwd.as_deref(), globals.config.as_deref()) {
        Ok(opened) => opened,
        Err(error) => {
            tracing::error!(%error, "solve: no project");
            let _ = writeln!(err, "error: {error}");
            return EXIT_USAGE;
        }
    };
    solve_project(&project, &scope, &args, globals.format, out, err)
}

/// Solve the selected packages of `project`; skips fresh results.
///
/// With `Format::Json` the per-solver lines are replaced by one report on `out`.
pub fn solve_project(
    project: &Project,
    scope: &[usize],
    args: &SolveArgs,
    format: Format,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8 {
    let mut all = Vec::new();
    for pkg in &project.packages {
        match parse_package(pkg, err) {
            Some(p) => all.push(p),
            None => return EXIT_FAILED,
        }
    }
    let names: Vec<String> = project.packages.iter().map(package_name).collect();
    let sources = package_sources(project, &names, &all);
    let table = UnitTable::builtin();
    let (resolver, resolve_errors) = Resolver::new(sources.clone());
    for e in &resolve_errors {
        tracing::warn!(code = e.code, message = %e.message, "solve: resolution diagnostic");
    }
    let mut evaluator = Evaluator::new(&resolver, &table, &sources);
    let configs = config_map(project);
    let run = RunEnv {
        configs: &configs,
        project,
        args,
        resolver: &resolver,
        units: &table,
        sources: &sources,
    };
    let mut failed = false;
    let mut usage = false;
    let mut entries: Vec<Entry> = Vec::new();
    let mut sink = Vec::new();
    let json = format == Format::Json;
    for &i in scope {
        for target in targets(&project.packages[i], &names[i], &all[i], args) {
            let lines: &mut dyn Write = if json { &mut sink } else { &mut *out };
            let outcome = solve_one(
                &target,
                &mut evaluator,
                &table,
                &run,
                &mut entries,
                lines,
                err,
            );
            let (is_usage, code, message) = match outcome {
                Outcome::Ok => continue,
                Outcome::Failed { code, message } => (false, code, message),
                Outcome::Usage { code, message } => (true, code, message),
            };
            if is_usage {
                usage = true;
            } else {
                failed = true;
            }
            let mut entry = Entry::new(&target.path, Status::Failed);
            entry.code = Some(code);
            entry.message = Some(message);
            entries.push(entry);
        }
    }
    if entries.is_empty() {
        tracing::info!("solve: no solvers selected");
        if !json {
            let _ = writeln!(out, "no solvers to run");
        }
    }
    if json {
        let report = Report {
            schema: REPORT_SCHEMA,
            ok: !(usage || failed),
            solvers: &entries,
        };
        match serde_json::to_string_pretty(&report) {
            Ok(text) => {
                let _ = writeln!(out, "{text}");
            }
            Err(error) => {
                tracing::error!(%error, "solve: could not render the report");
                let _ = writeln!(err, "error: could not render the report: {error}");
                return EXIT_FAILED;
            }
        }
    }
    if usage {
        EXIT_USAGE
    } else if failed {
        EXIT_FAILED
    } else {
        EXIT_OK
    }
}

/// Evaluate every `in` binding of `t` with `evaluator`; the error is a (code, message) pair.
fn eval_inputs(
    t: &Target<'_>,
    evaluator: &mut Evaluator<'_, '_>,
    table: &UnitTable,
) -> Result<BTreeMap<String, InputValue>, (String, String)> {
    let mut inputs = BTreeMap::new();
    for member in &t.ast.members {
        let SolverMemberKind::In(binds) = &member.kind else {
            continue;
        };
        for Bind { name, value, .. } in binds {
            match evaluator.eval_input(t.ctx.clone(), value) {
                Ok(q) => {
                    inputs.insert(name.name.clone(), input_value(table, &q));
                }
                Err(d) => {
                    return Err((
                        d.code.to_owned(),
                        format!("input `{}`: {}", name.name, d.message),
                    ));
                }
            }
        }
    }
    Ok(inputs)
}

enum Outcome {
    Ok,
    Failed { code: String, message: String },
    Usage { code: String, message: String },
}

fn fail(err: &mut dyn Write, solver: &str, code: &str, message: &str) -> Outcome {
    tracing::error!(solver, code, %message, "solve: failed");
    let _ = writeln!(err, "error[{code}]: {solver}: {message}");
    Outcome::Failed {
        code: code.to_owned(),
        message: message.to_owned(),
    }
}

fn solve_one(
    t: &Target<'_>,
    evaluator: &mut Evaluator<'_, '_>,
    table: &UnitTable,
    run: &RunEnv<'_>,
    entries: &mut Vec<Entry>,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> Outcome {
    let (configs, project) = (run.configs, run.project);
    let decl: SolverDecl = match lower_solver(t.ast, &t.path) {
        Ok(d) => d,
        Err(errors) => {
            let first = errors
                .first()
                .map(|e| (e.code.to_string(), e.message.to_string()));
            for e in &errors {
                let _ = writeln!(err, "error[{}]: {}: {}", e.code, t.path, e.message);
                tracing::error!(solver = %t.path, code = e.code, "solve: bad declaration");
            }
            let (code, message) = first.unwrap_or_default();
            return Outcome::Failed { code, message };
        }
    };
    let inputs = match eval_inputs(t, evaluator, table) {
        Ok(inputs) => inputs,
        Err((code, message)) => return fail(err, &t.path, &code, &message),
    };
    let request = Request::new(t.path.clone(), inputs);
    let script = t.package_dir.join(&decl.script);
    let results_path = out_path(&script);
    let mut digests = CurrentDigests {
        input_digest: input_digest(&request),
        script_digest: String::new(),
        dep_digests: BTreeMap::new(),
    };
    match read_digest(&script) {
        Ok(d) => digests.script_digest = d,
        Err(e) => {
            return fail(
                err,
                &t.path,
                "S005",
                &format!("script {}: {e}", script.display()),
            );
        }
    }
    for dep in &decl.deps {
        match read_digest(&t.package_dir.join(dep)) {
            Ok(d) => {
                digests.dep_digests.insert(dep.clone(), d);
            }
            Err(e) => return fail(err, &t.path, "S005", &format!("dep {dep}: {e}")),
        }
    }
    let mut file = match ResultFile::load(&results_path) {
        Ok(f) => f,
        Err(e) => return fail(err, &t.path, "S005", &e.to_string()),
    };
    match file.lookup(&t.path, None, &digests) {
        Lookup::Fresh(_) => {
            tracing::info!(solver = %t.path, "solve: result is fresh, not running");
            let _ = writeln!(out, "fresh {}", t.path);
            entries.push(Entry::new(&t.path, Status::Fresh));
            return solve_variants(
                &VariantCtx {
                    t,
                    script: &script,
                    digests: &digests,
                    decl: &decl,
                },
                run,
                entries,
                out,
                err,
            );
        }
        Lookup::Stale(_) => tracing::info!(solver = %t.path, "solve: result is stale, running"),
        Lookup::Missing => tracing::info!(solver = %t.path, "solve: no result, running"),
    }
    let response = match transport::run_solver(configs, &t.path, &script, &request, &project.root) {
        Ok(r) => r,
        Err(e) => {
            let outcome = fail(err, &t.path, e.code(), &e.to_string());
            return match outcome {
                Outcome::Failed { code, message }
                    if matches!(
                        e,
                        TransportError::UnknownSolver { .. } | TransportError::BadTemplate { .. }
                    ) =>
                {
                    Outcome::Usage { code, message }
                }
                other => other,
            };
        }
    };
    if let Err(message) = check_outputs(&decl, &response.outputs, table) {
        return fail(err, &t.path, "S005", &message);
    }
    let digests_for_variants = digests.clone();
    file.upsert(SolveResult {
        solver: t.path.clone(),
        input_digest: digests.input_digest,
        script_digest: digests.script_digest,
        dep_digests: digests.dep_digests,
        version: response.version,
        variant: None,
        inputs: request.inputs,
        outputs: response.outputs,
        diagnostics: response.diagnostics,
    });
    if let Err(e) = file.save(&results_path) {
        return fail(err, &t.path, "S005", &e.to_string());
    }
    tracing::info!(solver = %t.path, path = %results_path.display(), "solve: result stored");
    let _ = writeln!(out, "solved {}", t.path);
    entries.push(Entry::new(&t.path, Status::Solved));
    solve_variants(
        &VariantCtx {
            t,
            script: &script,
            digests: &digests_for_variants,
            decl: &decl,
        },
        run,
        entries,
        out,
        err,
    )
}

// frob:ticket 01M4V3F2DQTCGEAPVNHQDVP8JQ
/// Check a response's outputs against the declaration (solvers.md 3): every declared
/// output present and, when its kind is built in, a quantity of that dimension.
fn check_outputs(
    decl: &SolverDecl,
    outputs: &BTreeMap<String, String>,
    table: &UnitTable,
) -> Result<(), String> {
    let kinds = KindTable::builtin();
    for (name, declared) in &decl.outputs {
        let Some(lit) = outputs.get(name) else {
            tracing::warn!(solver = %decl.path, output = %name, "solve: response lacks a declared output");
            return Err(format!("response is missing declared output `{name}`"));
        };
        let Ok(want) = kinds.eval_expr(&declared.kind) else {
            tracing::debug!(output = %name, kind = %declared.kind, "solve: kind not built in, value kind unchecked");
            continue;
        };
        let lit = lit.trim();
        let split = lit
            .find(|c: char| !(c.is_ascii_digit() || matches!(c, '.' | '-' | '+' | 'e' | 'E')))
            .unwrap_or(lit.len());
        let (number, unit) = lit.split_at(split);
        if number.parse::<f64>().is_err() {
            return Err(format!("output `{name}` is not a quantity: `{lit}`"));
        }
        let unit = unit.trim();
        let dimension = if unit.is_empty() {
            cadetta_units::Dim::default().dimension
        } else {
            match table.parse(unit) {
                Ok(r) => r.dim.dimension,
                Err(_) => return Err(format!("output `{name}` has unknown unit in `{lit}`")),
            }
        };
        if dimension != want.dim.dimension {
            tracing::warn!(solver = %decl.path, output = %name, kind = %declared.kind, %lit, "solve: output has the wrong kind");
            return Err(format!(
                "output `{name}` is `{lit}`, not a {}",
                declared.kind
            ));
        }
    }
    Ok(())
}

/// The shared, read-only context of one `solve` run.
struct RunEnv<'a> {
    configs: &'a BTreeMap<String, SolverConfig>,
    project: &'a Project,
    args: &'a SolveArgs,
    resolver: &'a Resolver<'a>,
    units: &'a UnitTable,
    sources: &'a [PackageSrc<'a>],
}

/// Every approved alternate of every part slot in the project, as a slot selection.
// frob:ticket 01M4TRMJMNWCMHCE2H3BJF002X
fn selections(run: &RunEnv<'_>) -> Vec<Selection> {
    let mut found = Vec::new();
    for pkg in run.sources {
        for module in &pkg.modules {
            for item in module.file.items() {
                let ItemKind::Component(c) = &item.kind else {
                    continue;
                };
                let owner = CanonPath::item(&pkg.name, &module.path, &c.name.name);
                for member in &c.members {
                    let MemberKind::Part(part) = &member.kind else {
                        continue;
                    };
                    for ctor in &part.alts {
                        match run.resolver.ctor_component(&pkg.name, &module.path, ctor) {
                            Some((alt, _)) => found.push(Selection {
                                owner: owner.clone(),
                                slot: part.name.name.clone(),
                                alt,
                            }),
                            None => tracing::warn!(
                                slot = %part.name.name, owner = %owner,
                                "solve: alternate does not resolve to a component"
                            ),
                        }
                    }
                }
            }
        }
    }
    found
}

/// The requests of each approved alternate, evaluated under that alternate's slot selection.
///
/// An alternate whose inputs fail to evaluate is not viable for this solver and is skipped;
/// those whose inputs equal the primary's are dropped later by the planner.
// frob:ticket 01M4TRMJMNWCMHCE2H3BJF002X
fn variant_requests(t: &Target<'_>, run: &RunEnv<'_>) -> Vec<VariantRequest> {
    let mut requests = Vec::new();
    for sel in selections(run) {
        let mut evaluator =
            Evaluator::new(run.resolver, run.units, run.sources).with_selection(sel.clone());
        match eval_inputs(t, &mut evaluator, run.units) {
            Ok(inputs) => requests.push(VariantRequest {
                variant: sel.key(),
                request: Request::new(t.path.clone(), inputs),
            }),
            Err((code, message)) => {
                tracing::warn!(solver = %t.path, variant = %sel.key(), %code, %message,
                    "solve: variant inputs do not evaluate, skipping");
            }
        }
    }
    tracing::info!(solver = %t.path, variants = requests.len(), "solve: variant inputs evaluated");
    requests
}

/// What `solve_variants` needs about the solver whose primary row was just handled.
struct VariantCtx<'a> {
    t: &'a Target<'a>,
    script: &'a Path,
    digests: &'a CurrentDigests,
    decl: &'a SolverDecl,
}

// frob:ticket 01M4YP8J587Y5AFZ4E4HZPVB08
/// Run and store one result per distinct changed input digest (solvers.md 4.1),
/// rejecting responses whose outputs fail `check_outputs` (S005) without storing them.
fn solve_variants(
    ctx: &VariantCtx<'_>,
    run: &RunEnv<'_>,
    entries: &mut Vec<Entry>,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> Outcome {
    let VariantCtx {
        t,
        script,
        digests,
        decl,
    } = *ctx;
    if !run.args.variants {
        return Outcome::Ok;
    }
    let results_path = out_path(script);
    let mut file = match ResultFile::load(&results_path) {
        Ok(f) => f,
        Err(e) => return fail(err, &t.path, "S005", &e.to_string()),
    };
    let jobs = variants::plan(&file, &t.path, digests, &variant_requests(t, run));
    let mut outcome = Outcome::Ok;
    for job in &jobs {
        match transport::run_solver(
            run.configs,
            &t.path,
            script,
            &job.request,
            &run.project.root,
        ) {
            Ok(response) => {
                if let Err(message) = check_outputs(decl, &response.outputs, run.units) {
                    outcome = fail(
                        err,
                        &t.path,
                        "S005",
                        &format!("variant [{}]: {message}", job.variants.join(", ")),
                    );
                    continue;
                }
                variants::store(&mut file, &t.path, digests, job, &response);
                let _ = writeln!(out, "solved {} [{}]", t.path, job.variants.join(", "));
                let mut entry = Entry::new(&t.path, Status::Solved);
                entry.variants = job.variants.clone();
                entries.push(entry);
            }
            Err(e) => {
                outcome = fail(err, &t.path, e.code(), &e.to_string());
            }
        }
    }
    if !jobs.is_empty()
        && let Err(e) = file.save(&results_path)
    {
        return fail(err, &t.path, "S005", &e.to_string());
    }
    outcome
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a one-package project whose solver appends a line to `marker` per run.
    fn fixture(tag: &str, mass: &str) -> (std::path::PathBuf, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!("cadetta-solve-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src")).unwrap();
        let marker = dir.join("marker");
        std::fs::write(
            dir.join("cadetta.toml"),
            "[package]\nname = \"acme.t\"\nversion = \"0.1.0\"\nas_of = \"2026-10-13\"\n\n\
             [solvers.\"acme.t::Box.size\"]\ncommand = [\"sh\", \"{script}\"]\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("src/lib.cdt"),
            format!(
                "model Box {{\n  m: Mass = {mass}\n  solver size {{\n  script \"size.sh\"\n    in m = m\n    out w: Length\n  }}\n}}\n"
            ),
        )
        .unwrap();
        std::fs::write(
            dir.join("size.sh"),
            format!(
                "echo run >> {}\necho '{{\"schema\":\"cadetta.solve/1\",\"outputs\":{{\"w\":\"2m\"}}}}'\n",
                marker.display()
            ),
        )
        .unwrap();
        (dir, marker)
    }

    fn solve(dir: &Path) -> (u8, String) {
        let globals = Globals {
            format: crate::Format::Text,
            cwd: Some(dir.to_owned()),
            config: None,
        };
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = run_solve(&[], &globals, &mut out, &mut err);
        let text = String::from_utf8(err).unwrap() + &String::from_utf8(out).unwrap();
        (code, text)
    }

    fn runs(marker: &Path) -> usize {
        std::fs::read_to_string(marker).map_or(0, |t| t.lines().count())
    }

    // frob:tests crates/cadetta-cli/src/cmd/solve.rs::run_solve
    #[test]
    fn missing_then_stale_results_run_the_solver_and_store() {
        let (dir, marker) = fixture("run", "2kg");
        let (code, text) = solve(&dir);
        assert_eq!(code, EXIT_OK, "{text}");
        assert_eq!(runs(&marker), 1);
        let stored = ResultFile::load(&dir.join("size.out.json")).unwrap();
        assert_eq!(stored.results[0].solver, "acme.t::Box.size");
        assert_eq!(stored.results[0].outputs["w"], "2m");
        assert_eq!(stored.results[0].inputs["m"].lit, "2kg");
        // Changing an input makes the stored result stale: it runs again.
        let src = std::fs::read_to_string(dir.join("src/lib.cdt")).unwrap();
        std::fs::write(dir.join("src/lib.cdt"), src.replace("2kg", "3kg")).unwrap();
        assert_eq!(solve(&dir).0, EXIT_OK);
        assert_eq!(runs(&marker), 2);
    }

    // frob:ticket 01M4V3F2DQTCGEAPVNHQDVP8JQ
    // frob:tests crates/cadetta-cli/src/cmd/solve.rs::check_outputs
    #[test]
    fn missing_and_wrong_kind_outputs_are_s005_and_store_nothing() {
        for (tag, body) in [("outmissing", "{}"), ("outwrongkind", "{\"w\":\"3kg\"}")] {
            let (dir, _marker) = fixture(tag, "2kg");
            std::fs::write(
                dir.join("size.sh"),
                format!("echo '{{\"schema\":\"cadetta.solve/1\",\"outputs\":{body}}}'\n"),
            )
            .unwrap();
            let (code, text) = solve(&dir);
            assert_eq!(code, EXIT_FAILED, "{tag}: {text}");
            assert!(text.contains("S005"), "{tag}: {text}");
            assert!(text.contains("acme.t::Box.size"), "{tag}: {text}");
            assert!(
                !dir.join("size.out.json").exists(),
                "{tag}: stored a result"
            );
        }
    }

    // frob:ticket 01M4TRMJMNWCMHCE2H3BJF002X
    // frob:tests crates/cadetta-cli/src/cmd/solve.rs::variant_requests
    #[test]
    fn variants_flag_solves_each_changed_alternate_once() {
        let (dir, marker) = fixture("variants", "2kg");
        std::fs::write(
            dir.join("src/lib.cdt"),
            "interface Motor { mass: Mass }\n\
             model MotorA { mass = 2kg }\n\
             model MotorB { mass = 5kg }\n\
             model MotorC { mass = 2kg }\n\
             model Box {\n  part motor: Motor = MotorA alt [MotorB, MotorC]\n  \
             solver size {\n    script \"size.sh\"\n    in m = motor.mass\n    out w: Length\n  }\n}\n",
        )
        .unwrap();
        let globals = Globals {
            format: crate::Format::Text,
            cwd: Some(dir.clone()),
            config: None,
        };
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = run_solve(&["--variants".to_owned()], &globals, &mut out, &mut err);
        let text = String::from_utf8(err).unwrap() + &String::from_utf8(out).unwrap();
        assert_eq!(code, EXIT_OK, "{text}");
        // The primary and MotorB (5kg) run; MotorC (2kg) equals the primary's inputs.
        assert_eq!(runs(&marker), 2, "{text}");
        let stored = ResultFile::load(&dir.join("size.out.json")).unwrap();
        let keys: Vec<_> = stored.results.iter().map(|r| r.variant.clone()).collect();
        assert_eq!(keys.len(), 2, "{keys:?}");
        let alt = stored.results.iter().find(|r| r.variant.is_some()).unwrap();
        assert!(
            alt.variant
                .as_deref()
                .unwrap()
                .ends_with("Box/motor=acme.t::MotorB")
        );
        assert_eq!(alt.inputs["m"].lit, "5kg");
    }

    // frob:ticket 01M4YP8J587Y5AFZ4E4HZPVB08
    // frob:tests crates/cadetta-cli/src/cmd/solve.rs::solve_variants
    #[test]
    fn variant_outputs_missing_or_wrong_kind_are_s005_and_not_stored() {
        let (dir, _marker) = fixture("varbad", "2kg");
        std::fs::write(
            dir.join("src/lib.cdt"),
            "interface Motor { mass: Mass }\n\
             model MotorA { mass = 2kg }\n\
             model MotorB { mass = 5kg }\n\
             model Box {\n  part motor: Motor = MotorA alt [MotorB]\n  \
             solver size {\n    script \"size.sh\"\n    in m = motor.mass\n    out w: Length\n  }\n}\n",
        )
        .unwrap();
        // Good for the primary (2kg), bad for the variant (5kg).
        std::fs::write(
            dir.join("size.sh"),
            "if grep -q 5kg; then echo '{\"schema\":\"cadetta.solve/1\",\"outputs\":{\"w\":\"3kg\"}}'; \
             else echo '{\"schema\":\"cadetta.solve/1\",\"outputs\":{\"w\":\"1m\"}}'; fi\n",
        )
        .unwrap();
        let globals = Globals {
            format: crate::Format::Text,
            cwd: Some(dir.clone()),
            config: None,
        };
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = run_solve(&["--variants".to_owned()], &globals, &mut out, &mut err);
        let text = String::from_utf8(err).unwrap();
        assert_eq!(code, EXIT_FAILED, "{text}");
        assert!(text.contains("S005") && text.contains("variant"), "{text}");
        let stored = ResultFile::load(&dir.join("size.out.json")).unwrap();
        assert!(
            stored.results.iter().all(|r| r.variant.is_none()),
            "stored a bad variant"
        );
    }

    // frob:tests crates/cadetta-cli/src/cmd/solve.rs::run_solve
    #[test]
    fn fresh_results_run_nothing_and_exit_zero() {
        let (dir, marker) = fixture("fresh", "2kg");
        assert_eq!(solve(&dir).0, EXIT_OK);
        let (code, text) = solve(&dir);
        assert_eq!(code, EXIT_OK, "{text}");
        assert_eq!(runs(&marker), 1);
        assert!(text.contains("fresh acme.t::Box.size"));
    }

    fn solve_json(dir: &Path) -> (u8, serde_json::Value, String) {
        let globals = Globals {
            format: Format::Json,
            cwd: Some(dir.to_owned()),
            config: None,
        };
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = run_solve(&[], &globals, &mut out, &mut err);
        let v = serde_json::from_slice(&out).unwrap_or_else(|e| panic!("{e}: {out:?}"));
        (code, v, String::from_utf8(err).unwrap())
    }

    fn validate(v: &serde_json::Value) {
        let schema: serde_json::Value =
            serde_json::from_str(include_str!("../../../../docs/schemas/solve-report.json"))
                .unwrap();
        let validator = jsonschema::validator_for(&schema).unwrap();
        assert!(validator.is_valid(v), "{v}");
    }

    // frob:ticket 01M4TZKF0EKQ5E4N9KTNGEACDP
    // frob:tests crates/cadetta-cli/src/cmd/solve.rs::solve_project
    #[test]
    fn json_report_lists_solved_then_fresh_and_validates() {
        let (dir, _marker) = fixture("json", "2kg");
        let (code, v, _) = solve_json(&dir);
        assert_eq!(code, EXIT_OK);
        validate(&v);
        assert_eq!(v["ok"], true);
        assert_eq!(v["solvers"][0]["solver"], "acme.t::Box.size");
        assert_eq!(v["solvers"][0]["status"], "solved");
        let (_, v, _) = solve_json(&dir);
        validate(&v);
        assert_eq!(v["solvers"][0]["status"], "fresh");
    }

    // frob:ticket 01M4TZKF0EKQ5E4N9KTNGEACDP
    // frob:tests crates/cadetta-cli/src/cmd/solve.rs::solve_project
    #[test]
    fn json_report_records_failures_with_code() {
        let (dir, _marker) = fixture("jsonfail", "2kg");
        std::fs::remove_file(dir.join("size.sh")).unwrap();
        let (code, v, err) = solve_json(&dir);
        assert_eq!(code, EXIT_FAILED);
        validate(&v);
        assert_eq!(v["ok"], false);
        assert_eq!(v["solvers"][0]["status"], "failed");
        assert_eq!(v["solvers"][0]["code"], "S005");
        assert!(err.contains("S005"));
    }

    // frob:tests crates/cadetta-cli/src/cmd/solve.rs::glob_match
    #[test]
    fn globs_match_solver_paths() {
        assert!(glob_match("acme.t::*", "acme.t::Box.size"));
        assert!(glob_match("*.size", "acme.t::Box.size"));
        assert!(!glob_match("acme.u::*", "acme.t::Box.size"));
    }

    // frob:tests crates/cadetta-cli/src/cmd/solve.rs::parse_args
    #[test]
    fn variants_flag_is_accepted() {
        assert!(parse_args(&["--variants".to_owned()]).unwrap().variants);
        assert_eq!(parse_args(&["a::*".to_owned()]).unwrap().globs, ["a::*"]);
    }

    // frob:tests crates/cadetta-cli/src/cmd/solve.rs::package_sources
    #[test]
    fn enums_are_not_double_indexed_by_the_resolver() {
        let (dir, _marker) = fixture("enum", "2kg");
        let lib = dir.join("src/lib.cdt");
        let src = std::fs::read_to_string(&lib).unwrap();
        std::fs::write(&lib, format!("enum Grade {{ A, B }}\n{src}")).unwrap();
        let (project, _) = Project::open(Some(&dir), None).unwrap();
        let names: Vec<String> = project.packages.iter().map(package_name).collect();
        let all: Vec<_> = project
            .packages
            .iter()
            .map(|p| parse_package(p, &mut Vec::new()).unwrap())
            .collect();
        let sources = package_sources(&project, &names, &all);
        let (_, errors) = Resolver::new(sources);
        assert!(errors.iter().all(|e| e.code != "E003"), "{errors:?}");
    }
}
