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

use cadetta_adapter::bindings::{BindingSet, R013, R014, RefTarget, ScopeCandidate, mangle_all};
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
use crate::config::{Adapter, BindMode, EmitPoint as ConfigPoint};
use crate::output::Format;
use crate::pipeline::analyze;
use crate::project::{PackageDir, Project};
use crate::scope::Scope as RunScope;
use crate::{EXIT_FAIL, EXIT_OK, EXIT_USAGE, Globals};

const USAGE: &str = "usage: cadetta emit [ADAPTER...] [--module PATH] [GLOB...]";

/// Version recorded in the `sw-equations` header line.
const SW_EQUATIONS_VERSION: &str = "0.1.0";

/// Native unit used when neither the locator nor the adapter declares one.
pub(crate) const DEFAULT_NATIVE_UNIT: &str = "mm";

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
    /// Record schema tag, always `cadetta.diag/1` (lints.md 1.1).
    pub schema: &'static str,
    /// Severity word; every emit diagnostic withholds its file, so `error`.
    pub severity: &'static str,
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
    /// A bindings configuration error that stops the run with exit 2 (adapters.md 3); not reported.
    #[serde(skip)]
    pub config_error: Option<String>,
}

pub(crate) fn diag(code: &str, message: impl Into<String>, path: Option<&Path>) -> EmitDiag {
    EmitDiag {
        schema: crate::output::DIAG_SCHEMA,
        severity: "error",
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

pub(crate) fn config_point(p: ConfigPoint) -> PolicyPoint {
    match p {
        ConfigPoint::Nominal => PolicyPoint::Nominal,
        ConfigPoint::Mid => PolicyPoint::Mid,
        ConfigPoint::Mmc => PolicyPoint::Mmc,
        ConfigPoint::Lmc => PolicyPoint::Lmc,
    }
}

pub(crate) fn attr_point(member: &Member) -> Option<PolicyPoint> {
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
pub(crate) fn negate_text(text: &str) -> String {
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
pub(crate) fn locators_for<'a>(set: &'a BindingSet, path: &Path) -> Option<&'a [Locator]> {
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

pub(crate) fn kind_of(value: &Value) -> ValueKind {
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

/// A member's citation as emit and names print it: display name plus locator, if any.
// frob:ticket 01M4VB644Y6YPK6YTBP4T5YM3Y
pub(crate) fn citation_text(member: &Member) -> Option<String> {
    member.citation.as_ref().map(|c| match &c.locator {
        // frob:ticket 01M4VB62VJQV9AKSSQME8NWW6Y
        Some(l) => format!("{} {l}", c.display_name()),
        None => c.display_name(),
    })
}

/// The members offered to scope binding: non-analysis, valued, accepted by the adapter scope.
// frob:ticket 01M4VB644Y6YPK6YTBP4T5YM3Y
pub(crate) fn scope_candidates(ir: &Ir, scope: &AdapterScope) -> Vec<ScopeCandidate> {
    ir.components
        .iter()
        .flat_map(|c| &c.members)
        .filter(|m| !m.analysis && m.value.is_some() && scope.accepts(&m.path.to_string()))
        .map(|m| ScopeCandidate {
            path: m.path.clone(),
            kind: m.kind,
            // sw-equations cannot express Text; such members carry no numeric value.
            text: m.value.is_none(),
        })
        .collect()
}

/// Everything one adapter's emit needs from its package.
struct Job<'a> {
    adapter: &'a Adapter,
    set: &'a BindingSet,
    ir: &'a Ir,
    config_point: PolicyPoint,
    scope: &'a AdapterScope,
    run: &'a RunScope,
    /// Canonical paths of the package's `Text` parameters.
    texts: &'a BTreeSet<String>,
    bindings_digest: String,
}

/// The built output of one adapter: the text plus the diagnostics met on the way.
struct Built {
    text: Option<String>,
    diagnostics: Vec<EmitDiag>,
}

pub(crate) fn native_unit(adapter: &Adapter, loc: &Locator) -> String {
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

/// True for a diagnostic that withholds the file and fails the run (Q004, R013, R014).
fn is_withholding(d: &EmitDiag) -> bool {
    d.code == "Q004" || d.code == R013 || d.code == R014
}

/// R014 for every bound `Text` parameter this run must emit (emitters/sw-equations.md 1).
///
/// Must-emit means: bound by a non-alternate locator of this adapter, accepted by the adapter
/// and run filters, and not a `let`; anything else is silently not emitted.
// frob:ticket 01M4V7WC7QPQ4RH2902T8PTE0A
fn text_diagnostics(job: &Job<'_>, diagnostics: &mut Vec<EmitDiag>) {
    for member in job.ir.components.iter().flat_map(|c| &c.members) {
        let canonical = member.path.to_string();
        if member.analysis
            || !job.texts.contains(&canonical)
            || !job.scope.accepts(&canonical)
            || !job.run.contains(&canonical)
        {
            continue;
        }
        let Some(loc) = locators_for(job.set, &member.path)
            .and_then(|ls| ls.iter().find(|l| l.for_alternate.is_none()))
        else {
            continue;
        };
        let message = format!(
            "{R014}: `{canonical}` is a Text parameter bound to `{}`; an equations file cannot express Text",
            loc.locator
        );
        tracing::warn!(path = %canonical, locator = %loc.locator, "emit: bound Text parameter must be emitted (R014)");
        diagnostics.push(diag(R014, message, Some(&member.path)));
    }
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

    let paths: Vec<Path> = members
        .iter()
        .filter(|m| {
            // A path bound to an explicit global creates no name (adapters.md 3.1).
            job.set
                .get(&m.path)
                .is_none_or(|b| !b.locators.iter().any(|l| l.kind == LocatorKind::Global))
        })
        .map(|m| m.path.clone())
        .collect();
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
        let citation = citation_text(member);
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
    text_diagnostics(job, &mut diagnostics);
    if diagnostics.iter().any(is_withholding) {
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
pub(crate) fn written_for(
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

/// Write `text` to `path` unless the file already equals it modulo the header line (adapters.md 2).
///
/// Returns the text now on disk, so the reported digest matches the file; a skipped rewrite
/// keeps the existing header.
// frob:ticket 01M4W47VHXH1PBPKTF87FQCBH8
fn write_file(path: &FsPath, text: &str) -> std::io::Result<String> {
    if let Ok(existing) = std::fs::read_to_string(path)
        && sw_equations::same_modulo_header(&existing, text)
    {
        tracing::info!(file = %path.display(), "emit: unchanged modulo header, not rewritten");
        return Ok(existing);
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, text)?;
    Ok(text.to_owned())
}

/// The digested design IR of one package from an analysis (an empty design when it has none).
// frob:ticket 01M4SPWTDD069AXXQBXD4T6ATZ
pub(crate) fn package_ir(
    analysis: &crate::pipeline::Analysis,
    pkg: &PackageDir,
) -> Result<Ir, String> {
    let id = package_id(pkg);
    let mut ir = analysis
        .packages
        .iter()
        .find(|p| p.name == id.name)
        .map_or_else(|| Ir::new(Flavour::Design, id.clone()), |p| p.ir.clone());
    digest_project(&mut ir).map_err(|e| format!("could not digest the IR: {e}"))?;
    Ok(ir)
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
        let ir = match package_ir(&analysis, pkg) {
            Ok(ir) => ir,
            Err(message) => return fail(err, &message),
        };
        let texts = analysis
            .packages
            .iter()
            .find(|l| l.name == package_id(pkg).name)
            .map(|l| l.text_params.clone())
            .unwrap_or_default();
        emit_package_texts(pkg, &ir, &texts, &parsed, &run_scope, &mut report);
        if let Some(message) = report.config_error.take() {
            return fail(err, &message);
        }
    }
    let failed = report.diagnostics.iter().any(is_withholding);
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

/// Why an adapter's bindings could not be loaded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LoadError {
    /// The bindings file or a locator is invalid: a configuration error with no code (adapters.md 3).
    Config(String),
    /// A lint-level failure (R013) that stops the adapter.
    Diag(EmitDiag),
}

impl LoadError {
    /// The error as a diagnostic; a configuration error keeps the legacy R005 code for callers
    /// that do not yet stop with exit 2.
    pub(crate) fn into_diag(self) -> EmitDiag {
        match self {
            Self::Config(message) => diag("R005", message, None),
            Self::Diag(d) => d,
        }
    }
}

/// Load an adapter's bindings file (missing means nothing bound) and apply `bind = scope`.
///
/// Returns the set and the file text, or the `LoadError` that stops the adapter.
// frob:ticket 01M4SPWTDD069AXXQBXD4T6ATZ
// frob:ticket 01M4V1X594YSHKE55103F67WS1
pub(crate) fn load_bindings(
    pkg: &PackageDir,
    adapter: &Adapter,
    ir: &Ir,
) -> Result<(BindingSet, String), LoadError> {
    let bindings_file = pkg.dir.join(adapter.bindings_path());
    let (mut set, bindings_text) = match std::fs::read_to_string(&bindings_file) {
        Ok(text) => match BindingSet::load_str(adapter.id(), &text) {
            Ok(set) => (set, text),
            Err(e) => {
                let message = format!("{}: {e}", bindings_file.display());
                tracing::error!(%message, "bindings: invalid file or locator");
                return Err(LoadError::Config(message));
            }
        },
        Err(_) => {
            tracing::info!(file = %bindings_file.display(), "bindings: no file, nothing bound");
            (BindingSet::default(), String::new())
        }
    };
    // frob:ticket 01M4VB640810PZEZHQJ1T5WFTK
    if adapter.bind == BindMode::Scope {
        let scope = AdapterScope::new(adapter.modules.clone(), adapter.paths.clone());
        let package = pkg.config.package.as_ref().and_then(|p| p.name.as_deref());
        let candidates = scope_candidates(ir, &scope);
        match set.bind_scope(
            &candidates,
            package.unwrap_or_default(),
            &adapter.prefix,
            false,
        ) {
            Ok(n) => tracing::info!(adapter = %adapter.id(), n, "bind = scope applied"),
            Err(e) => {
                tracing::warn!(error = %e, "native name collision (R013)");
                return Err(LoadError::Diag(diag(R013, e.to_string(), None)));
            }
        }
    }
    Ok((set, bindings_text))
}

/// Run emit for every selected adapter of one package, appending to `report`.
pub(crate) fn emit_package(
    pkg: &PackageDir,
    ir: &Ir,
    parsed: &EmitArgs,
    run_scope: &RunScope,
    report: &mut EmitReport,
) {
    emit_package_texts(pkg, ir, &BTreeSet::new(), parsed, run_scope, report);
}

/// [`emit_package`] that also knows the package's `Text` parameters, so R014 can fire.
// frob:ticket 01M4V7WC7QPQ4RH2902T8PTE0A
pub(crate) fn emit_package_texts(
    pkg: &PackageDir,
    ir: &Ir,
    texts: &BTreeSet<String>,
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
        let (set, bindings_text) = match load_bindings(pkg, adapter, ir) {
            Ok(loaded) => loaded,
            Err(LoadError::Config(message)) => {
                report.config_error = Some(message);
                return;
            }
            Err(LoadError::Diag(d)) => {
                report.diagnostics.push(d);
                continue;
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
            texts,
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
            Ok(on_disk) => {
                tracing::info!(file = %target.display(), "emit: equations up to date");
                report.files.push(EmitFile {
                    adapter: adapter.id().to_owned(),
                    path: rel,
                    digest: digest_text(on_disk.as_bytes()),
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
            "[\"acme.plate::X.a__b\"]\nlocators = [ { locator = \"D1@S\", kind = \"dimension\", native_unit = \"mm\" } ]\n\n[\"acme.plate::X__a.b\"]\nlocators = [ { locator = \"D2@S\", kind = \"dimension\", native_unit = \"mm\" } ]\n",
            "",
        );
        let (code, out, _) = run(&dir, &[]);
        assert_eq!(code, EXIT_FAIL, "{out}");
        assert!(out.contains("R013"), "{out}");
        assert!(!dir.0.join("build/solidworks/equations.txt").exists());
    }

    // frob:tests crates/cadetta-cli/src/cmd/emit.rs::run_emit
    #[test]
    fn explicit_binding_resolves_name_collision() {
        let dir = project(
            "model X {\n  a__b: Length = 1mm\n}\nmodel X__a {\n  b: Length = 2mm\n}\n",
            "[\"acme.plate::X.a__b\"]\nlocators = [ { locator = \"P\", kind = \"global\", native_unit = \"mm\" }, { locator = \"D1@S\", kind = \"dimension\", native_unit = \"mm\" } ]\n\n[\"acme.plate::X__a.b\"]\nlocators = [ { locator = \"D2@S\", kind = \"dimension\", native_unit = \"mm\" } ]\n",
            "",
        );
        let (code, out, _) = run(&dir, &[]);
        assert_eq!(code, 0, "{out}");
        assert!(!out.contains("R013"), "{out}");
    }

    // frob:ticket 01M4V1X594YSHKE55103F67WS1
    // frob:tests crates/cadetta-cli/src/cmd/emit.rs::load_bindings
    #[test]
    fn invalid_bindings_config_error_exits_two() {
        let bad = [
            "[\"acme.plate::CoverPlate.thickness\"]\nlocators = [ { locator = \"\", kind = \"global\" } ]\n",
            "[\"acme.plate::CoverPlate.thickness\"]\nlocators = [ { locator = \"W\", kind = \"global\", config = \"\" } ]\n",
            "[\"acme.plate::CoverPlate.thickness\"]\nlocators = [ { locator = \"W\", kind = \"global\", sign = 1, ref_to = \"X\" } ]\n",
        ];
        for text in bad {
            let dir = project(SOURCE, text, "");
            let (code, out, err) = run(&dir, &[]);
            assert_eq!(code, EXIT_USAGE, "{text}: {out} {err}");
            assert!(err.contains("equations.toml"), "names the path: {err}");
            assert!(!out.contains("R005"), "no diagnostic code: {out}");
        }
        let ok = project(SOURCE, BINDINGS, "");
        assert_eq!(run(&ok, &[]).0, EXIT_OK);
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

    // frob:ticket 01M4SWN8G3QTQC5YK07VN1YQYC
    // frob:tests crates/cadetta-cli/src/cmd/emit.rs::run_emit
    #[test]
    fn json_validates_against_emit_schema() {
        let schema: serde_json::Value =
            serde_json::from_str(include_str!("../../../../docs/schemas/emit.json")).unwrap();
        let validator = jsonschema::validator_for(&schema).unwrap();
        let ok = project(SOURCE, BINDINGS, "");
        let bad = project(
            "model A {\n  w: Length = 1/3mm\n}\n",
            "[\"acme.plate::A.w\"]\nlocators = [ { locator = \"W\", kind = \"global\", native_unit = \"mm\" } ]\n",
            "precision = 2\n",
        );
        for dir in [&ok, &bad] {
            let globals = Globals {
                format: Format::Json,
                cwd: Some(dir.0.clone()),
                config: None,
            };
            let (mut out, mut err) = (Vec::new(), Vec::new());
            run_emit(&[], &globals, &mut out, &mut err);
            let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
            assert!(validator.is_valid(&v), "{v}");
        }
    }

    // frob:tests crates/cadetta-cli/src/cmd/emit.rs::emit_package
    #[test]
    fn bind_scope_emits_every_in_scope_path_and_writes_no_bindings() {
        let src = "model CoverPlate {\n  thickness: Length = 6mm\n  offset: Length = 2mm\n  let twice = thickness * 2\n}\n";
        let dir = project(src, BINDINGS, "bind = \"scope\"\n");
        let before = std::fs::read_to_string(dir.0.join("bindings/equations.toml")).unwrap();
        let (code, out, e) = run(&dir, &[]);
        assert_eq!((code, e.as_str()), (EXIT_OK, ""), "{out}");
        let text = std::fs::read_to_string(dir.0.join("build/solidworks/equations.txt")).unwrap();
        assert!(
            text.contains("\"Plate_Thk\" = 6mm"),
            "explicit name kept: {text}"
        );
        assert!(text.contains("\"Off_X\" = -2mm"), "{text}");
        assert!(
            !text.contains("twice") && !text.contains("12mm"),
            "let emitted: {text}"
        );
        assert_eq!(
            std::fs::read_to_string(dir.0.join("bindings/equations.toml")).unwrap(),
            before
        );
    }

    // frob:tests crates/cadetta-cli/src/cmd/emit.rs::emit_package
    #[test]
    fn bind_scope_creates_globals_for_unbound_paths() {
        let dir = project(SOURCE, "", "bind = \"scope\"\n");
        let (code, out, _) = run(&dir, &[]);
        assert_eq!(code, EXIT_OK, "{out}");
        let text = std::fs::read_to_string(dir.0.join("build/solidworks/equations.txt")).unwrap();
        for v in ["6mm", "2mm", "9mm"] {
            assert!(text.contains(v), "{v} missing: {text}");
        }
    }

    // frob:tests crates/cadetta-cli/src/cmd/emit.rs::write_file
    #[test]
    fn header_only_difference_is_not_rewritten() {
        let dir = project(SOURCE, BINDINGS, "");
        let target = dir.0.join("build/solidworks/equations.txt");
        assert_eq!(run(&dir, &[]).0, EXIT_OK);
        let fresh = std::fs::read_to_string(&target).unwrap();
        let (header, body) = fresh.split_once('\n').unwrap();
        assert!(header.starts_with("' cadetta emit"), "{header}");
        let stale = format!("' cadetta emit sw-equations 0.0.1 ir=blake3:old\n{body}");
        std::fs::write(&target, &stale).unwrap();
        assert_eq!(run(&dir, &[]).0, EXIT_OK);
        assert_eq!(std::fs::read_to_string(&target).unwrap(), stale);
        let changed = stale.replace("6mm", "7mm");
        std::fs::write(&target, &changed).unwrap();
        assert_eq!(run(&dir, &[]).0, EXIT_OK);
        assert_eq!(std::fs::read_to_string(&target).unwrap(), fresh);
    }
}
