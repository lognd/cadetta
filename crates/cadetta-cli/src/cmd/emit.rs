// frob:ticket 01M4SPWTA1PQV15S63741V30XT
//! `cadetta emit [ADAPTER...] [--module PATH] [GLOB...]`: run emit for the configured adapters
//! and write their outputs (config-cli.md 3, adapters.md 3, emitters/sw-equations.md).
//!
//! Only the `sw-equations` built-in is implemented here; `json`, `csv` and external adapters
//! (those with a `command`) are skipped with a logged notice. Q004 (a value not representable
//! at the declared precision) and R013 (two paths creating one native name) give exit 1.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::io::Write;
use std::path::{Path as FsPath, PathBuf};

use cadetta_adapter::bindings::{BindingSet, R013, RefTarget, mangle_all};
use cadetta_adapter::builtin::sw_equations::{
    self, ADAPTER as SW_EQUATIONS, DEFAULT_PRECISION, Header, Line, PathBlock, Rhs, ValueKind,
};
use cadetta_adapter::policy::{
    Designed, EmitPoint as PolicyPoint, Ratio, Role, build_request, resolve_point,
};
use cadetta_adapter::scope::Scope as AdapterScope;
use cadetta_ir::digest::digest_project;
use cadetta_ir::model::{
    Flavour, Locator, LocatorKind, LocatorMode, Member, Nominal, Project as Ir, Sign, Value,
};
use cadetta_ir::path::Path;
use cadetta_units::{Magnitude, UnitTable};
use serde::Serialize;

use crate::cmd::ir::package_id;
use crate::config::{Adapter, EmitPoint as ConfigPoint};
use crate::output::Format;
use crate::pipeline::analyze;
use crate::project::{PackageDir, Project};
use crate::scope::Scope as RunScope;
use crate::{EXIT_FAIL, EXIT_OK, EXIT_USAGE, Globals};

const USAGE: &str = "usage: cadetta emit [ADAPTER...] [--module PATH] [GLOB...]";

/// Version recorded in the `sw-equations` header line.
const SW_EQUATIONS_VERSION: &str = "0.1.0";

/// Native unit used when neither the locator nor the adapter declares one.
const DEFAULT_NATIVE_UNIT: &str = "mm";

/// The parsed command line of `cadetta emit`.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct EmitArgs {
    /// Adapter ids to run; empty means every configured adapter.
    pub adapters: Vec<String>,
    /// `--module` prefixes (repeatable).
    pub modules: Vec<String>,
    /// Canonical-path globs.
    pub globs: Vec<String>,
}

/// Parse the words after `emit`; a word containing `*` or `::` is a glob, the rest are adapter ids.
pub fn parse_args(args: &[String]) -> Result<EmitArgs, String> {
    let mut parsed = EmitArgs::default();
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--module" => parsed.modules.push(
                it.next()
                    .cloned()
                    .ok_or_else(|| "`--module` needs a value".to_owned())?,
            ),
            flag if flag.starts_with("--") => return Err(format!("unknown flag `{flag}`")),
            word if word.contains('*') || word.contains("::") || word.contains('.') => {
                parsed.globs.push(word.to_owned());
            }
            word => parsed.adapters.push(word.to_owned()),
        }
    }
    Ok(parsed)
}

/// A diagnostic produced while emitting.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EmitDiag {
    /// Rule code, e.g. `Q004` or `R013`.
    pub code: String,
    /// Human message.
    pub message: String,
    /// The canonical path concerned, when there is one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

/// One file written by emit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EmitFile {
    /// The adapter id that produced it.
    pub adapter: String,
    /// The file path, relative to the package directory.
    pub path: String,
    /// `blake3:<hex>` of the written bytes.
    pub digest: String,
}

/// The `emit.json` document (config-cli.md 3).
#[derive(Debug, Default, Serialize)]
pub struct EmitReport {
    /// Files written.
    pub files: Vec<EmitFile>,
    /// Diagnostics, errors first by occurrence.
    pub diagnostics: Vec<EmitDiag>,
}

fn diag(code: &str, message: impl Into<String>, path: Option<&Path>) -> EmitDiag {
    EmitDiag {
        code: code.to_owned(),
        message: message.into(),
        path: path.map(ToString::to_string),
    }
}

/// Convert an SI magnitude to a ratio of `factor` units; `Err` when pi remains or it overflows.
fn to_native(mag: &Magnitude, factor: &Magnitude) -> Result<Magnitude, String> {
    let m = mag.div(factor).map_err(|e| e.to_string())?;
    if m.pi_exp() != 0 {
        return Err("the value is not a rational multiple of the native unit".to_owned());
    }
    Ok(m)
}

fn to_ratio(m: &Magnitude) -> Result<Ratio, String> {
    let num: i128 = m
        .rational()
        .numer()
        .to_string()
        .parse()
        .map_err(|_| "the value is too large to emit".to_owned())?;
    let den: i128 = m
        .rational()
        .denom()
        .to_string()
        .parse()
        .map_err(|_| "the value is too large to emit".to_owned())?;
    Ok(Ratio::new(num, den))
}

/// The designed nominal and limits of `value` in the unit whose SI factor is `factor`.
fn designed(value: &Value, factor: &Magnitude) -> Result<Designed, String> {
    let Nominal::Exact(nominal) = &value.nominal else {
        return Err("the value is inexact (Q006) and cannot be emitted exactly".to_owned());
    };
    let nom = to_native(&nominal.0, factor)?;
    let (mut lo, mut hi) = (nom.clone(), nom.clone());
    if let Some(tol) = &value.tol {
        for term in &tol.terms {
            for (acc, delta, name) in [(&mut lo, &term.lo, "lo"), (&mut hi, &term.hi, "hi")] {
                let d = to_native(&delta.0, factor)?;
                match acc.add(&d) {
                    cadetta_units::Sum::Exact(m) => *acc = m,
                    cadetta_units::Sum::Inexact(_) => {
                        return Err(format!("the tolerance {name} limit is inexact"));
                    }
                }
            }
        }
    }
    Ok(Designed {
        nominal: to_ratio(&nom)?,
        min: to_ratio(&lo)?,
        max: to_ratio(&hi)?,
    })
}

fn config_point(p: ConfigPoint) -> PolicyPoint {
    match p {
        ConfigPoint::Nominal => PolicyPoint::Nominal,
        ConfigPoint::Mid => PolicyPoint::Mid,
        ConfigPoint::Mmc => PolicyPoint::Mmc,
        ConfigPoint::Lmc => PolicyPoint::Lmc,
    }
}

fn attr_point(member: &Member) -> Option<PolicyPoint> {
    match member.attrs.get("emit").map(String::as_str) {
        Some("nominal") => Some(PolicyPoint::Nominal),
        Some("mid") => Some(PolicyPoint::Mid),
        Some("mmc") => Some(PolicyPoint::Mmc),
        Some("lmc") => Some(PolicyPoint::Lmc),
        _ => None,
    }
}

fn role_of(member: &Member) -> Option<Role> {
    match member.role.as_deref() {
        Some("hole") => Some(Role::Hole),
        Some("shaft") => Some(Role::Shaft),
        _ => None,
    }
}

/// Prefix `-` on a written decimal (`6.35mm` -> `-6.35mm`), keeping zero unsigned.
fn negate_text(text: &str) -> String {
    if let Some(rest) = text.strip_prefix('-') {
        return rest.to_owned();
    }
    let numeric: String = text
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    if numeric.chars().all(|c| c == '0' || c == '.') {
        text.to_owned()
    } else {
        format!("-{text}")
    }
}

/// Replace every `[N]` index with `[*]`, for matching a member against a wildcard binding.
fn wildcard(path: &str) -> (String, bool) {
    let mut out = String::new();
    let mut all_zero = true;
    let mut rest = path;
    while let Some(open) = rest.find('[') {
        let Some(close) = rest[open..].find(']') else {
            break;
        };
        let inner = &rest[open + 1..open + close];
        if inner.chars().all(|c| c.is_ascii_digit()) && !inner.is_empty() {
            if inner != "0" {
                all_zero = false;
            }
            out.push_str(&rest[..open]);
            out.push_str("[*]");
        } else {
            out.push_str(&rest[..open + close + 1]);
        }
        rest = &rest[open + close + 1..];
    }
    out.push_str(rest);
    (out, all_zero)
}

/// The locators bound to `path`: an exact binding, or a `[*]` binding at instance 0 only.
fn locators_for<'a>(set: &'a BindingSet, path: &Path) -> Option<&'a [Locator]> {
    if let Some(b) = set.get(path) {
        return Some(&b.locators);
    }
    let (wild, zero) = wildcard(&path.to_string());
    if wild == path.to_string() || !zero {
        return None;
    }
    set.entries()
        .iter()
        .find(|b| b.path.to_string() == wild)
        .map(|b| b.locators.as_slice())
}

fn kind_of(value: &Value) -> ValueKind {
    if !value.dim.dimension.is_dimensionless() || value.angle != 0 || value.dim.angle != 0 {
        return ValueKind::Quantity;
    }
    match &value.nominal {
        Nominal::Exact(e) if e.0.rational().denom().to_string() == "1" && e.0.pi_exp() == 0 => {
            ValueKind::Count
        }
        _ => ValueKind::Ratio,
    }
}

/// Everything one adapter's emit needs from its package.
struct Job<'a> {
    adapter: &'a Adapter,
    set: &'a BindingSet,
    ir: &'a Ir,
    config_point: PolicyPoint,
    scope: &'a AdapterScope,
    run: &'a RunScope,
    bindings_digest: String,
}

/// The built output of one adapter: the text plus the diagnostics met on the way.
struct Built {
    text: Option<String>,
    diagnostics: Vec<EmitDiag>,
}

fn native_unit(adapter: &Adapter, loc: &Locator) -> String {
    if !loc.native_unit.is_empty() {
        loc.native_unit.clone()
    } else {
        adapter
            .native_unit
            .clone()
            .unwrap_or_else(|| DEFAULT_NATIVE_UNIT.to_owned())
    }
}

/// A pure alias `a = b` maps `a` to `b` so `a`'s ref-mode dimension points at `b`'s global.
fn alias_map(ir: &Ir) -> HashMap<Path, Path> {
    let mut map = HashMap::new();
    for member in ir.components.iter().flat_map(|c| &c.members) {
        let [dep] = member.deps.as_slice() else {
            continue;
        };
        let name = dep.to_string();
        let short = name.rsplit(['.', ':']).next().unwrap_or(&name);
        if member
            .expr
            .as_deref()
            .is_some_and(|e| e.trim() == name || e.trim() == short)
        {
            map.insert(member.path.clone(), dep.clone());
        }
    }
    map
}

/// Build the `sw-equations` file for one adapter.
fn build_sw_equations(job: &Job<'_>) -> Built {
    let mut diagnostics = Vec::new();
    let table = UnitTable::builtin();
    let precision = job.adapter.precision.map_or(DEFAULT_PRECISION, u32::from);
    let members: Vec<&Member> = job
        .ir
        .components
        .iter()
        .flat_map(|c| &c.members)
        .filter(|m| {
            let canonical = m.path.to_string();
            !m.analysis
                && m.value.is_some()
                && job.scope.accepts(&canonical)
                && job.run.contains(&canonical)
                && locators_for(job.set, &m.path).is_some()
        })
        .collect();
    tracing::debug!(adapter = %job.adapter.id(), members = members.len(), "emit: bound members selected");

    let paths: Vec<Path> = members.iter().map(|m| m.path.clone()).collect();
    let mangled: HashMap<Path, String> = match mangle_all(&paths, &job.adapter.prefix) {
        Ok(pairs) => pairs.into_iter().collect(),
        Err(e) => {
            tracing::warn!(error = %e, "emit: native name collision (R013)");
            diagnostics.push(diag(R013, e.to_string(), None));
            return Built {
                text: None,
                diagnostics,
            };
        }
    };
    let alias_of = alias_map(job.ir);

    let mut blocks: BTreeMap<String, PathBlock> = BTreeMap::new();
    for member in &members {
        let (Some(value), Some(locators)) = (&member.value, locators_for(job.set, &member.path))
        else {
            continue;
        };
        let mut block = PathBlock {
            path: member.path.to_string(),
            doc: member
                .doc
                .as_deref()
                .map(|d| d.lines().map(str::to_owned).collect())
                .unwrap_or_default(),
            globals: Vec::new(),
            dimensions: Vec::new(),
        };
        let kind = kind_of(value);
        let point = resolve_point(&[attr_point(member)], job.config_point);
        let citation = member.citation.as_ref().map(|c| match &c.locator {
            Some(l) => format!("{} {l}", c.name),
            None => c.name.clone(),
        });
        let mut own_global_written = false;
        for loc in locators {
            if loc.for_alternate.is_some() {
                tracing::debug!(path = %member.path, "emit: alternate-only locator skipped");
                continue;
            }
            let unit = native_unit(job.adapter, loc);
            let written = match written_for(&table, value, member, &unit, kind, point, precision) {
                Ok(Some(w)) => w,
                Ok(None) => continue,
                Err((code, message)) => {
                    tracing::warn!(path = %member.path, code, %message, "emit: value withheld");
                    diagnostics.push(diag(code, message, Some(&member.path)));
                    continue;
                }
            };
            let comment = sw_equations::trailing_comment(
                Some(&value.authored),
                &written,
                point,
                citation.as_deref(),
            );
            let line_value = if loc.sign == Some(Sign::Minus) {
                negate_text(&written)
            } else {
                written.clone()
            };
            match (loc.kind, loc.mode) {
                (LocatorKind::Global, _) => block.globals.push(Line {
                    lhs: loc.locator.clone(),
                    rhs: Rhs::Value(line_value),
                    comment,
                }),
                (_, LocatorMode::Ref) => {
                    let own = mangled.get(&member.path).cloned().unwrap_or_default();
                    let target = job.set.resolve_ref(&member.path, loc, &alias_of, &own);
                    let name = match target {
                        RefTarget::Global(g) | RefTarget::AliasOf(g) => g,
                        RefTarget::OwnGlobal(g) => {
                            if !own_global_written {
                                own_global_written = true;
                                block.globals.push(Line {
                                    lhs: g.clone(),
                                    rhs: Rhs::Value(line_value),
                                    comment,
                                });
                            }
                            g
                        }
                    };
                    block.dimensions.push(Line {
                        lhs: loc.locator.clone(),
                        rhs: Rhs::Ref(name),
                        comment: String::new(),
                    });
                }
                (_, LocatorMode::Value) => block.dimensions.push(Line {
                    lhs: loc.locator.clone(),
                    rhs: Rhs::Value(line_value),
                    comment,
                }),
            }
        }
        if !block.globals.is_empty() || !block.dimensions.is_empty() {
            blocks.insert(block.path.clone(), block);
        }
    }
    if diagnostics
        .iter()
        .any(|d| d.code == "Q004" || d.code == R013)
    {
        return Built {
            text: None,
            diagnostics,
        };
    }

    let graph: BTreeMap<String, Vec<String>> = job
        .ir
        .components
        .iter()
        .flat_map(|c| &c.members)
        .map(|m| {
            (
                m.path.to_string(),
                m.deps.iter().map(ToString::to_string).collect(),
            )
        })
        .collect();
    let emitted: BTreeSet<String> = blocks.keys().cloned().collect();
    let order = match sw_equations::order_paths(&graph, &emitted) {
        Ok(o) => o,
        Err(e) => {
            diagnostics.push(diag("D003", e.to_string(), None));
            return Built {
                text: None,
                diagnostics,
            };
        }
    };
    let mut ordered: Vec<PathBlock> = order.iter().filter_map(|p| blocks.remove(p)).collect();
    ordered.extend(blocks.into_values());
    let header = Header {
        adapter: format!("{SW_EQUATIONS} {SW_EQUATIONS_VERSION}"),
        ir: format!(
            "blake3:{}",
            job.ir.digest.as_ref().map(|d| d.0.as_str()).unwrap_or("")
        ),
        bindings: job.bindings_digest.clone(),
    };
    Built {
        text: Some(sw_equations::emit(&header, &ordered)),
        diagnostics,
    }
}

/// The written text of one value, `Ok(None)` for a Text value, `Err((code, message))` on Q004.
fn written_for(
    table: &UnitTable,
    value: &Value,
    member: &Member,
    unit: &str,
    kind: ValueKind,
    point: PolicyPoint,
    precision: u32,
) -> Result<Option<String>, (&'static str, String)> {
    let (unit_factor, unit_text) = if kind == ValueKind::Quantity {
        let resolved = table
            .parse(unit)
            .map_err(|e| ("Q004", format!("unit `{unit}`: {e}")))?;
        (resolved.factor, unit)
    } else {
        (Magnitude::new(cadetta_units::Rational::one(), 0), "")
    };
    let d = designed(value, &unit_factor).map_err(|m| ("Q004", m))?;
    let req = build_request(
        &member.path.to_string(),
        &d,
        point,
        role_of(member),
        unit_text,
    )
    .map_err(|e| (e.code(), e.to_string()))?;
    sw_equations::written_value(kind, &req, precision).map_err(|e| ("Q004", e.to_string()))
}

fn digest_text(bytes: &[u8]) -> String {
    format!("blake3:{}", blake3::hash(bytes).to_hex())
}

fn write_file(path: &FsPath, text: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, text)
}

/// Run `cadetta emit`; `args` are the words after the verb. Returns the exit code.
pub fn run_emit(
    args: &[String],
    globals: &Globals,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8 {
    let fail = |err: &mut dyn Write, message: &str| {
        tracing::error!(%message, "emit failed");
        let _ = writeln!(err, "error: {message}");
        EXIT_USAGE
    };
    let parsed = match parse_args(args) {
        Ok(p) => p,
        Err(message) => return fail(err, &format!("{message}; {USAGE}")),
    };
    let run_scope = match RunScope::new(&parsed.modules, &parsed.globs) {
        Ok(s) => s,
        Err(e) => return fail(err, &format!("{e}; {USAGE}")),
    };
    let (project, scope) = match Project::open(globals.cwd.as_deref(), globals.config.as_deref()) {
        Ok(opened) => opened,
        Err(e) => return fail(err, &e.to_string()),
    };
    let known: BTreeSet<&str> = scope
        .iter()
        .flat_map(|i| project.packages[*i].config.adapters.iter().map(Adapter::id))
        .collect();
    if let Some(unknown) = parsed.adapters.iter().find(|a| !known.contains(a.as_str())) {
        return fail(err, &format!("no adapter `{unknown}` in cadetta.toml"));
    }
    let analysis = analyze(&project, &scope);
    if analysis.halted {
        return fail(err, "a syntax error stopped analysis; run `cadetta check`");
    }
    let mut report = EmitReport::default();
    for index in &scope {
        let pkg = &project.packages[*index];
        let id = package_id(pkg);
        let mut ir = analysis
            .packages
            .iter()
            .find(|p| p.name == id.name)
            .map_or_else(|| Ir::new(Flavour::Design, id.clone()), |p| p.ir.clone());
        if let Err(e) = digest_project(&mut ir) {
            return fail(err, &format!("could not digest the IR: {e}"));
        }
        emit_package(pkg, &ir, &parsed, &run_scope, &mut report);
    }
    let failed = report
        .diagnostics
        .iter()
        .any(|d| d.code == "Q004" || d.code == R013);
    let rendered = match globals.format {
        Format::Json => serde_json::to_string_pretty(&report).map_err(|e| e.to_string()),
        _ => Ok(render_text(&report)),
    };
    match rendered {
        Ok(text) => {
            let _ = writeln!(out, "{}", text.trim_end());
        }
        Err(e) => return fail(err, &format!("could not render the report: {e}")),
    }
    if failed { EXIT_FAIL } else { EXIT_OK }
}

fn render_text(report: &EmitReport) -> String {
    let mut s = String::new();
    for f in &report.files {
        s.push_str(&format!("wrote {} ({})\n", f.path, f.adapter));
    }
    for d in &report.diagnostics {
        match &d.path {
            Some(p) => s.push_str(&format!("{}: {} [{p}]\n", d.code, d.message)),
            None => s.push_str(&format!("{}: {}\n", d.code, d.message)),
        }
    }
    if report.files.is_empty() && report.diagnostics.is_empty() {
        s.push_str("nothing to emit\n");
    }
    s
}

/// Run emit for every selected adapter of one package, appending to `report`.
fn emit_package(
    pkg: &PackageDir,
    ir: &Ir,
    parsed: &EmitArgs,
    run_scope: &RunScope,
    report: &mut EmitReport,
) {
    let cfg_point = config_point(pkg.config.emit.point);
    for adapter in &pkg.config.adapters {
        if !parsed.adapters.is_empty() && !parsed.adapters.iter().any(|a| a == adapter.id()) {
            continue;
        }
        if adapter.command.is_some() {
            tracing::info!(adapter = %adapter.id(), "emit: external adapters are not run by the CLI yet");
            continue;
        }
        if adapter.name != SW_EQUATIONS {
            tracing::info!(adapter = %adapter.id(), "emit: built-in not implemented, skipped");
            continue;
        }
        let bindings_file = pkg.dir.join(adapter.bindings_path());
        let (set, bindings_text) = match std::fs::read_to_string(&bindings_file) {
            Ok(text) => match BindingSet::load_str(adapter.id(), &text) {
                Ok(set) => (set, text),
                Err(e) => {
                    report.diagnostics.push(diag("R005", e.to_string(), None));
                    continue;
                }
            },
            Err(_) => {
                tracing::info!(file = %bindings_file.display(), "emit: no bindings file, nothing bound");
                (BindingSet::default(), String::new())
            }
        };
        let scope = AdapterScope::new(adapter.modules.clone(), adapter.paths.clone());
        let job = Job {
            adapter,
            set: &set,
            ir,
            config_point: cfg_point,
            scope: &scope,
            run: run_scope,
            bindings_digest: digest_text(bindings_text.as_bytes()),
        };
        let built = build_sw_equations(&job);
        report.diagnostics.extend(built.diagnostics);
        let Some(text) = built.text else {
            continue;
        };
        let rel = adapter
            .out
            .clone()
            .unwrap_or_else(|| format!("build/solidworks/{}.txt", adapter.id()));
        let target: PathBuf = pkg.dir.join(&rel);
        match write_file(&target, &text) {
            Ok(()) => {
                tracing::info!(file = %target.display(), "emit: wrote equations");
                report.files.push(EmitFile {
                    adapter: adapter.id().to_owned(),
                    path: rel,
                    digest: digest_text(text.as_bytes()),
                });
            }
            Err(e) => report.diagnostics.push(diag(
                "E000",
                format!("could not write {}: {e}", target.display()),
                None,
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A self-removing scratch directory (no extra dev-dependency).
    struct Dir(PathBuf);
    impl Dir {
        fn new() -> Self {
            static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
            let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let p = std::env::temp_dir().join(format!("cadetta-emit-{}-{n}", std::process::id()));
            std::fs::create_dir_all(&p).unwrap();
            Self(p)
        }
    }
    impl Drop for Dir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn project(source: &str, bindings: &str, adapter_extra: &str) -> Dir {
        let dir = Dir::new();
        let toml = format!(
            "[package]\nname = \"acme.plate\"\nversion = \"0.1.0\"\n[[adapter]]\nname = \"sw-equations\"\nid = \"equations\"\n{adapter_extra}"
        );
        std::fs::write(dir.0.join("cadetta.toml"), toml).unwrap();
        std::fs::create_dir_all(dir.0.join("src")).unwrap();
        std::fs::write(dir.0.join("src/lib.cdt"), source).unwrap();
        std::fs::create_dir_all(dir.0.join("bindings")).unwrap();
        std::fs::write(dir.0.join("bindings/equations.toml"), bindings).unwrap();
        dir
    }

    fn run(dir: &Dir, args: &[&str]) -> (u8, String, String) {
        let args: Vec<String> = args.iter().map(|a| (*a).to_owned()).collect();
        let globals = Globals {
            format: Format::Text,
            cwd: Some(dir.0.clone()),
            config: None,
        };
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = run_emit(&args, &globals, &mut out, &mut err);
        (
            code,
            String::from_utf8(out).unwrap(),
            String::from_utf8(err).unwrap(),
        )
    }

    const SOURCE: &str = "model CoverPlate {\n  thickness: Length = 6mm\n  offset: Length = 2mm\n  loose: Length = 9mm\n}\n";
    const BINDINGS: &str = "[\"acme.plate::CoverPlate.thickness\"]\nlocators = [ { locator = \"Plate_Thk\", kind = \"global\", native_unit = \"mm\" } ]\n\n[\"acme.plate::CoverPlate.offset\"]\nlocators = [ { locator = \"Off_X\", kind = \"global\", native_unit = \"mm\", sign = -1 } ]\n";

    // frob:tests crates/cadetta-cli/src/cmd/emit.rs::run_emit
    #[test]
    fn writes_equations_for_bound_members_only() {
        let dir = project(SOURCE, BINDINGS, "");
        let (code, out, e) = run(&dir, &[]);
        assert_eq!((code, e.as_str()), (EXIT_OK, ""), "{out}");
        let text = std::fs::read_to_string(dir.0.join("build/solidworks/equations.txt")).unwrap();
        assert!(
            text.starts_with("' cadetta emit sw-equations 0.1.0 ir=blake3:"),
            "{text}"
        );
        assert!(text.contains("\"Plate_Thk\" = 6mm"), "{text}");
        assert!(text.contains("\"Off_X\" = -2mm"), "sign=-1 negates: {text}");
        assert!(!text.contains("9mm"), "unbound member emitted: {text}");
        assert!(out.contains("build/solidworks/equations.txt"), "{out}");
    }

    // frob:tests crates/cadetta-cli/src/cmd/emit.rs::run_emit
    #[test]
    fn honours_out_and_adapter_selection() {
        let dir = project(SOURCE, BINDINGS, "out = \"build/eq.txt\"\n");
        let (code, _, _) = run(&dir, &["equations"]);
        assert_eq!(code, EXIT_OK);
        assert!(dir.0.join("build/eq.txt").is_file());
    }

    // frob:tests crates/cadetta-cli/src/cmd/emit.rs::run_emit
    #[test]
    fn unrepresentable_value_exits_one() {
        let dir = project(
            "model A {\n  w: Length = 1/3mm\n}\n",
            "[\"acme.plate::A.w\"]\nlocators = [ { locator = \"W\", kind = \"global\", native_unit = \"mm\" } ]\n",
            "precision = 2\n",
        );
        let (code, out, _) = run(&dir, &[]);
        assert_eq!(code, EXIT_FAIL, "{out}");
        assert!(out.contains("Q004"), "{out}");
    }

    // frob:tests crates/cadetta-cli/src/cmd/emit.rs::run_emit
    #[test]
    fn native_name_collision_exits_one() {
        let dir = project(
            "model X {\n  a__b: Length = 1mm\n}\nmodel X__a {\n  b: Length = 2mm\n}\n",
            "[\"acme.plate::X.a__b\"]\nlocators = [ { locator = \"P\", kind = \"global\", native_unit = \"mm\" } ]\n\n[\"acme.plate::X__a.b\"]\nlocators = [ { locator = \"Q\", kind = \"global\", native_unit = \"mm\" } ]\n",
            "",
        );
        let (code, out, _) = run(&dir, &[]);
        assert_eq!(code, EXIT_FAIL, "{out}");
        assert!(out.contains("R013"), "{out}");
        assert!(!dir.0.join("build/solidworks/equations.txt").exists());
    }

    // frob:tests crates/cadetta-cli/src/cmd/emit.rs::run_emit
    #[test]
    fn usage_errors_exit_two() {
        let dir = project(SOURCE, BINDINGS, "");
        assert_eq!(run(&dir, &["nosuch"]).0, EXIT_USAGE);
        assert_eq!(run(&dir, &["--bogus"]).0, EXIT_USAGE);
        assert_eq!(run(&dir, &["--module"]).0, EXIT_USAGE);
    }

    // frob:tests crates/cadetta-cli/src/cmd/emit.rs::parse_args
    #[test]
    fn parses_adapters_modules_and_globs() {
        let args: Vec<String> = ["equations", "--module", "acme", "acme.plate::*"]
            .map(String::from)
            .to_vec();
        let p = parse_args(&args).unwrap();
        assert_eq!(p.adapters, ["equations"]);
        assert_eq!(p.modules, ["acme"]);
        assert_eq!(p.globs, ["acme.plate::*"]);
    }
}
