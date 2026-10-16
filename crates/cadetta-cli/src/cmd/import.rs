// frob:ticket 01M4SPWTA1PQV15S63741V30XT
//! `cadetta import ADAPTER FILE... [--assume-unit U]` and `import --accept`: adopt existing CAD
//! files (config-cli.md 3, adapters.md 7).
//!
//! Propose mode runs the configured external adapter's `read`, then writes only
//! `build/import/proposal.toml` and `proposal.cdt`. `--accept` turns the reviewed proposal into
//! `bindings/<id>.toml`, `cad/imported/<Component>.cdt` and an `adopted` lock record; existing
//! files are never overwritten, and a re-run adds only what is not yet bound. Import never
//! exits 1: success is 0 and every failure is a usage-class error (2).

use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::path::{Path as FsPath, PathBuf};

use cadetta_adapter::bindings::BindingSet;
use cadetta_adapter::import::accept::plan as accept_plan;
use cadetta_adapter::import::naming::component_file;
use cadetta_adapter::import::propose::{
    Item, PROPOSAL_DIR, PROPOSAL_TOML, Proposal, Status, propose, render_model, write_proposal,
};
use cadetta_adapter::import::update::{self, Definition, Source};
use cadetta_adapter::rpc::client::ProcessClient;
use cadetta_adapter::rpc::types::{ReadParams, ReadResult, ReadSource};
use cadetta_adapter::sync::Entry;
use cadetta_ir::dim::DimWire;
use cadetta_ir::model::{
    Exact, Implicit, Locator, LocatorKind, LocatorMode, Nominal, Project as Ir, Provenance, Sign,
};
use cadetta_ir::observed::{ObservedBuilder, ObservedParam};
use cadetta_obligations::lock::{Adopted, Lock, Record, SyncedValue};
use cadetta_units::{Dim, Magnitude, Rational};
use serde::Serialize;
use serde_json::Value;

use crate::cmd::ir::package_id;
use crate::cmd::sync::{normalize, observed_geometry};
use crate::config::Adapter;
use crate::output::Format;
use crate::project::{PackageDir, Project};
use crate::{EXIT_OK, EXIT_USAGE, Globals};

const USAGE: &str = "usage: cadetta import ADAPTER FILE... [--assume-unit U] | import --accept | import ADAPTER FILE... --update [--write]";

/// The parsed command line of `cadetta import`.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct ImportArgs {
    /// The adapter id (first word), when given.
    pub adapter: Option<String>,
    /// The native files (remaining words).
    pub files: Vec<String>,
    /// `--assume-unit U`: the unit recorded for unitless native values.
    pub assume_unit: Option<String>,
    /// `--accept`: write bindings, `.cdt` and the lock from the reviewed proposal.
    pub accept: bool,
    /// `--update`: diff `.cdt` literals against the native files.
    pub update: bool,
    /// `--write`: with `--update`, write the edits.
    pub write: bool,
}

/// Parse the words after `import`.
pub fn parse_args(args: &[String]) -> Result<ImportArgs, String> {
    let mut parsed = ImportArgs::default();
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--assume-unit" => {
                parsed.assume_unit = Some(
                    it.next()
                        .cloned()
                        .ok_or_else(|| "`--assume-unit` needs a value".to_owned())?,
                );
            }
            "--accept" => parsed.accept = true,
            "--update" => parsed.update = true,
            "--write" => parsed.write = true,
            flag if flag.starts_with("--") => return Err(format!("unknown flag `{flag}`")),
            word if parsed.adapter.is_none() => parsed.adapter = Some(word.to_owned()),
            word => parsed.files.push(word.to_owned()),
        }
    }
    if parsed.accept && parsed.update {
        return Err("`--accept` and `--update` are exclusive".to_owned());
    }
    if parsed.write && !parsed.update {
        return Err("`--write` needs `--update`".to_owned());
    }
    if !parsed.accept && parsed.adapter.is_none() {
        return Err("an ADAPTER is required".to_owned());
    }
    if !parsed.accept && parsed.files.is_empty() {
        return Err("at least one FILE is required".to_owned());
    }
    Ok(parsed)
}

/// The `import.json` document (config-cli.md 3).
#[derive(Debug, Default, Serialize)]
pub struct ImportReport {
    /// `propose`, `accept` or `update`.
    pub mode: String,
    /// Files written, relative to the package directory.
    pub written: Vec<String>,
    /// Proposed or adopted canonical paths (or items) added.
    pub items: Vec<String>,
    /// Entries left unchanged, with the reason.
    pub unchanged: Vec<String>,
    /// Notes and adapter diagnostics.
    pub notes: Vec<String>,
}

fn rational_from(text: &str) -> Result<Rational, String> {
    let (n, d) = text.split_once('/').unwrap_or((text, "1"));
    let n: i128 = n.parse().map_err(|_| format!("bad numerator `{n}`"))?;
    let d: i128 = d.parse().map_err(|_| format!("bad denominator `{d}`"))?;
    Rational::new(n, d).map_err(|e| e.to_string())
}

fn nominal_from(v: &Value) -> Result<Nominal, String> {
    let r = v
        .get("r")
        .and_then(Value::as_str)
        .ok_or("nominal lacks `r` (inexact nominals cannot be imported)")?;
    let pi = v.get("pi").and_then(Value::as_i64).unwrap_or(0);
    let pi = i32::try_from(pi).map_err(|_| "pi exponent out of range".to_owned())?;
    Ok(Nominal::Exact(Exact(Magnitude::new(rational_from(r)?, pi))))
}

fn locator_from(v: &Value) -> Result<Locator, String> {
    let text = |k: &str| v.get(k).and_then(Value::as_str).map(str::to_owned);
    let locator = text("locator").ok_or("locator lacks `locator`")?;
    let kind = match text("kind").as_deref() {
        Some("global") => LocatorKind::Global,
        Some("dimension") => LocatorKind::Dimension,
        Some("property") => LocatorKind::Property,
        Some("frame") => LocatorKind::Frame,
        Some("instance") => LocatorKind::Instance,
        other => return Err(format!("locator `{locator}`: bad kind {other:?}")),
    };
    let mode = match text("mode").as_deref() {
        None | Some("value") => LocatorMode::Value,
        Some("ref") => LocatorMode::Ref,
        other => return Err(format!("locator `{locator}`: bad mode {other:?}")),
    };
    let sign = match v.get("sign").and_then(Value::as_i64) {
        None => None,
        Some(1) => Some(Sign::Plus),
        Some(-1) => Some(Sign::Minus),
        Some(n) => return Err(format!("locator `{locator}`: bad sign {n}")),
    };
    // frob:ticket 01M4VB640810PZEZHQJ1T5WFTK
    let implicit = match text("implicit").as_deref() {
        None => None,
        Some("global") => Some(Implicit::Global),
        Some("link") => Some(Implicit::Link),
        Some("frame") => Some(Implicit::Frame),
        Some("reference") => Some(Implicit::Reference),
        other => return Err(format!("locator `{locator}`: bad implicit {other:?}")),
    };
    Ok(Locator {
        locator,
        kind,
        mode,
        native_unit: text("native_unit").unwrap_or_default(),
        ref_to: text("ref_to"),
        for_alternate: text("for"),
        config: text("config"),
        sign,
        component: text("component"),
        implicit,
        tolerance: None,
    })
}

/// Decode an adapter's `observed` JSON (one project, or an array of them) into one observed
/// project per native file, applying `assume_unit` to unitless values.
pub fn decode_observed(
    observed: &Value,
    adapter: &str,
    package: cadetta_ir::model::PackageId,
    assume_unit: Option<&str>,
) -> Result<Vec<Ir>, String> {
    let projects: Vec<&Value> = match observed {
        Value::Array(a) => a.iter().collect(),
        v => vec![v],
    };
    let mut by_file: BTreeMap<String, ObservedBuilder> = BTreeMap::new();
    let mut linked: BTreeMap<(String, String), &'static str> = BTreeMap::new();
    for project in projects {
        let empty = Vec::new();
        let bindings = project
            .get("bindings")
            .and_then(Value::as_array)
            .unwrap_or(&empty);
        let binding_of = |path: &str| {
            bindings
                .iter()
                .find(|b| b.get("path").and_then(Value::as_str) == Some(path))
        };
        let comps = project
            .get("components")
            .and_then(Value::as_array)
            .unwrap_or(&empty);
        for member in comps
            .iter()
            .filter_map(|c| c.get("members").and_then(Value::as_array))
            .flatten()
        {
            let Some(value) = member.get("value").filter(|v| !v.is_null()) else {
                continue;
            };
            let path = member
                .get("path")
                .and_then(Value::as_str)
                .ok_or("member lacks `path`")?;
            let imported = member
                .get("provenance")
                .and_then(|p| p.get("imported"))
                .ok_or_else(|| format!("member `{path}` lacks imported provenance"))?;
            let file = imported
                .get("file")
                .and_then(Value::as_str)
                .ok_or("provenance lacks `file`")?;
            let native_name = imported
                .get("native_name")
                .and_then(Value::as_str)
                .ok_or("provenance lacks `native_name`")?;
            let locators = binding_of(path)
                .and_then(|b| b.get("locators"))
                .and_then(Value::as_array)
                .ok_or_else(|| format!("member `{path}` has no binding"))?;
            let dim: DimWire =
                serde_json::from_value(value.get("dim").cloned().unwrap_or_default())
                    .map_err(|e| format!("member `{path}`: bad dim: {e}"))?;
            let unit_assumed = value
                .get("unit_assumed")
                .and_then(Value::as_str)
                .map(|u| assume_unit.unwrap_or(u).to_owned());
            if let Some(kind) = linked_kind(member, value) {
                tracing::debug!(file, native_name, kind, "import: linked property observed");
                linked.insert((file.to_owned(), native_name.to_owned()), kind);
            }
            let builder = builder_for(&mut by_file, &package, adapter, file)?;
            for loc in locators {
                let param = ObservedParam {
                    native_name: native_name.to_owned(),
                    locator: locator_from(loc)?,
                    nominal: nominal_from(value.get("nominal").unwrap_or(&Value::Null))?,
                    dim: Dim::from(&dim),
                    text: value
                        .get("authored")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_owned(),
                    expr: member
                        .get("expr")
                        .and_then(Value::as_str)
                        .map(str::to_owned),
                    unit_assumed: unit_assumed.clone(),
                    inexact_reason: if value.get("exact").and_then(Value::as_bool) == Some(false) {
                        Some(
                            value
                                .get("reason")
                                .and_then(Value::as_str)
                                .unwrap_or("not faithful")
                                .to_owned(),
                        )
                    } else {
                        None
                    },
                };
                builder.push(param).map_err(|e| e.to_string())?;
            }
        }
    }
    // frob:ticket 01M4ZCHT2YJ2Z7F2E9TR77QY6K
    let (frames, instances) = observed_geometry(observed);
    tracing::debug!(
        frames = frames.len(),
        instances = instances.len(),
        "import: observed geometry decoded"
    );
    for f in frames {
        let doc = f.document.clone();
        builder_for(&mut by_file, &package, adapter, &doc)?
            .push_frame(f)
            .map_err(|e| e.to_string())?;
    }
    for i in instances {
        let doc = i.document.clone();
        builder_for(&mut by_file, &package, adapter, &doc)?
            .push_instance(i)
            .map_err(|e| e.to_string())?;
    }
    let mut built: Vec<Ir> = by_file.into_values().map(ObservedBuilder::build).collect();
    mark_linked(&mut built, &linked);
    Ok(built)
}

/// How a property's value is linked rather than authored: `prp` (`$PRP:"..."` text) or
/// `equation` (the adapter's `equation_linked` flag); adapters.md 7.3.
fn linked_kind(member: &Value, value: &Value) -> Option<&'static str> {
    if member.get("equation_linked").and_then(Value::as_bool) == Some(true) {
        return Some("equation");
    }
    value
        .get("authored")
        .and_then(Value::as_str)
        .filter(|t| t.starts_with("$PRP:"))
        .map(|_| "prp")
}

/// Record each linked member's kind in its `linked` attribute, where `propose` reads it.
fn mark_linked(projects: &mut [Ir], linked: &BTreeMap<(String, String), &'static str>) {
    for member in projects
        .iter_mut()
        .flat_map(|p| &mut p.components)
        .flat_map(|c| &mut c.members)
    {
        if let Provenance::Imported {
            file, native_name, ..
        } = &member.provenance
            && let Some(kind) = linked.get(&(file.clone(), native_name.clone()))
        {
            member.attrs.insert("linked".to_owned(), (*kind).to_owned());
        }
    }
}

/// The per-file observed builder for `file`, created on first use.
// frob:ticket 01M4ZCHT2YJ2Z7F2E9TR77QY6K
fn builder_for<'a>(
    by_file: &'a mut BTreeMap<String, ObservedBuilder>,
    package: &cadetta_ir::model::PackageId,
    adapter: &str,
    file: &str,
) -> Result<&'a mut ObservedBuilder, String> {
    if !by_file.contains_key(file) {
        let b = ObservedBuilder::new(package.clone(), adapter, file).map_err(|e| e.to_string())?;
        by_file.insert(file.to_owned(), b);
    }
    Ok(by_file.get_mut(file).expect("inserted above"))
}

fn fail(err: &mut dyn Write, message: &str) -> u8 {
    tracing::error!(%message, "import failed");
    let _ = writeln!(err, "error: {message}");
    EXIT_USAGE
}

/// Run `cadetta import`; `args` are the words after the verb. Never returns 1.
pub fn run_import(
    args: &[String],
    globals: &Globals,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8 {
    let parsed = match parse_args(args) {
        Ok(p) => p,
        Err(message) => return fail(err, &format!("{message}; {USAGE}")),
    };
    let (project, scope) = match Project::open(globals.cwd.as_deref(), globals.config.as_deref()) {
        Ok(opened) => opened,
        Err(e) => return fail(err, &e.to_string()),
    };
    let result = if parsed.update {
        update_mode(&project, &scope, &parsed, globals)
    } else if parsed.accept {
        accept(&project, &scope, &parsed)
    } else {
        propose_mode(&project, &scope, &parsed, globals)
    };
    let report = match result {
        Ok(r) => r,
        Err(message) => return fail(err, &message),
    };
    let rendered = match globals.format {
        Format::Json => serde_json::to_string_pretty(&report).map_err(|e| e.to_string()),
        _ => Ok(render_text(&report)),
    };
    match rendered {
        Ok(text) => {
            let _ = writeln!(out, "{}", text.trim_end());
            EXIT_OK
        }
        Err(e) => fail(err, &format!("could not render the report: {e}")),
    }
}

fn render_text(r: &ImportReport) -> String {
    let mut s = String::new();
    for w in &r.written {
        s.push_str(&format!("wrote {w}\n"));
    }
    for i in r.items.iter().filter(|_| r.mode == "update") {
        s.push_str(&format!("{}\n", i.trim_end()));
    }
    for i in r.items.iter().filter(|_| r.mode != "update") {
        s.push_str(&format!(
            "  {} {i}\n",
            if r.mode == "accept" {
                "adopted"
            } else {
                "proposed"
            }
        ));
    }
    for u in &r.unchanged {
        s.push_str(&format!("unchanged {u}\n"));
    }
    for n in &r.notes {
        s.push_str(&format!("note: {n}\n"));
    }
    if r.mode == "propose" && !r.written.is_empty() {
        s.push_str("review build/import/proposal.toml, then run `cadetta import --accept`\n");
    }
    s
}

/// Resolve an adapter command relative to the package directory; there is no PATH search
/// (adapters.md 5), so a bare name such as `sh` names `<pkg>/sh`, never the system `sh`.
// frob:ticket 01M4YG7D22XEEK9M3X678B88BT
pub(crate) fn program(pkg: &PackageDir, command: &[String]) -> Option<(PathBuf, Vec<String>)> {
    let (first, rest) = command.split_first()?;
    let mut path = pkg.dir.join(first);
    if path.components().count() == 1 {
        // `Command` searches PATH for a path with no separator; anchor it explicitly.
        path = FsPath::new(".").join(path);
    }
    Some((path, rest.to_vec()))
}

/// The package and adapter a `bind`/`import` id (adapter id or built-in name) names.
// frob:ticket 01M4VB644Y6YPK6YTBP4T5YM3Y
pub(crate) fn find_adapter<'a>(
    project: &'a Project,
    scope: &[usize],
    id: &str,
) -> Option<(&'a PackageDir, &'a Adapter)> {
    scope.iter().find_map(|i| {
        let pkg = &project.packages[*i];
        pkg.config
            .adapters
            .iter()
            .find(|a| a.id() == id || a.name == id)
            .map(|a| (pkg, a))
    })
}

/// Spawn the adapter named in `parsed` and read its FILES (relative ones against `globals.cwd`).
fn read_adapter<'a>(
    project: &'a Project,
    scope: &[usize],
    parsed: &ImportArgs,
    globals: &Globals,
) -> Result<(&'a PackageDir, &'a Adapter, ReadResult), String> {
    let id = parsed.adapter.as_deref().unwrap_or_default();
    let (pkg, adapter) = find_adapter(project, scope, id)
        .ok_or_else(|| format!("no adapter `{id}` in cadetta.toml"))?;
    let command = adapter.command.as_deref().ok_or_else(|| {
        format!("adapter `{id}` has no `command`; only external adapters can read")
    })?;
    let (path, argv) =
        program(pkg, command).ok_or_else(|| format!("adapter `{id}` has an empty `command`"))?;
    let files: Vec<String> = parsed
        .files
        .iter()
        .map(|f| match &globals.cwd {
            Some(cwd) if !FsPath::new(f).is_absolute() => {
                cwd.join(f).to_string_lossy().into_owned()
            }
            _ => f.clone(),
        })
        .collect();
    tracing::info!(adapter = %id, files = files.len(), "import: reading");
    let mut client = ProcessClient::spawn_with_timeout(
        &path,
        &argv,
        adapter.version.as_deref(),
        Some(adapter.timeout()),
    )
    .map_err(|e| format!("R008: could not start adapter `{id}`: {e:?}"))?;
    let read = client
        .read(&ReadParams {
            source: ReadSource::Files(files),
        })
        .map_err(|e| format!("adapter `{id}` read failed: {e:?}"));
    if let Err(e) = client.shutdown() {
        tracing::warn!(error = ?e, "import: adapter shutdown failed");
    }
    Ok((pkg, adapter, read?))
}

fn propose_mode(
    project: &Project,
    scope: &[usize],
    parsed: &ImportArgs,
    globals: &Globals,
) -> Result<ImportReport, String> {
    let (pkg, adapter, read) = read_adapter(project, scope, parsed, globals)?;
    let observed = decode_observed(
        &read.observed,
        adapter.id(),
        package_id(pkg),
        parsed.assume_unit.as_deref(),
    )?;
    let mut proposal = propose(adapter.id(), &observed);
    if let Some(unit) = &parsed.assume_unit {
        for item in proposal.items.iter_mut().filter(|i| i.unit_assumed) {
            item.unit = Some(unit.clone());
        }
    }
    let written = write_proposal(&pkg.dir, &proposal).map_err(|e| e.to_string())?;
    let mut report = ImportReport {
        mode: "propose".to_owned(),
        ..ImportReport::default()
    };
    for p in written {
        report.written.push(rel(&pkg.dir, &p));
    }
    for item in &proposal.items {
        let tag = if item.status == Status::Skip {
            format!(
                "{}.{} (skipped: {})",
                item.component,
                item.path,
                item.reason.as_deref().unwrap_or("")
            )
        } else {
            format!("{}.{}", item.component, item.path)
        };
        report.items.push(tag);
    }
    for d in &read.diagnostics {
        report.notes.push(d.0.to_string());
    }
    Ok(report)
}

/// Observed CAD values by native locator: `(native file, normalized `{number}{unit}`)`.
fn observed_values(observed: &Value) -> BTreeMap<String, (String, String)> {
    let projects: Vec<&Value> = match observed {
        Value::Array(a) => a.iter().collect(),
        v => vec![v],
    };
    let mut out = BTreeMap::new();
    for project in projects {
        let members = project
            .get("components")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|c| c.get("members").and_then(Value::as_array))
            .flatten();
        let bindings: Vec<&Value> = project
            .get("bindings")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .collect();
        for member in members {
            let text = |v: Option<&Value>, k: &str| {
                v.and_then(|v| v.get(k))
                    .and_then(Value::as_str)
                    .map(str::to_owned)
            };
            let Some(path) = text(Some(member), "path") else {
                continue;
            };
            let value = member.get("value");
            let (Some(authored), Some(file)) = (
                text(value, "authored"),
                text(
                    member.get("provenance").and_then(|p| p.get("imported")),
                    "file",
                ),
            ) else {
                continue;
            };
            let assumed = text(value, "unit_assumed");
            let locators = bindings
                .iter()
                .filter(|b| b.get("path").and_then(Value::as_str) == Some(path.as_str()))
                .filter_map(|b| b.get("locators").and_then(Value::as_array))
                .flatten();
            for loc in locators {
                let Some(name) = text(Some(loc), "locator") else {
                    continue;
                };
                let unit = text(Some(loc), "native_unit")
                    .or_else(|| assumed.clone())
                    .unwrap_or_default();
                out.insert(name, (file.clone(), normalize(&authored, &unit)));
            }
        }
    }
    out
}

/// Every `.cdt` of the package (`src/` modules and `cad/imported/`) as `(relative path, text)`.
fn package_sources(pkg: &PackageDir) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = pkg
        .modules
        .iter()
        .map(|m| (rel(&pkg.dir, &m.file), m.source.clone()))
        .collect();
    let imported = pkg.dir.join("cad/imported");
    if let Ok(entries) = std::fs::read_dir(&imported) {
        let mut paths: Vec<PathBuf> = entries
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e == "cdt"))
            .collect();
        paths.sort();
        for path in paths {
            match std::fs::read_to_string(&path) {
                Ok(text) => out.push((rel(&pkg.dir, &path), text)),
                Err(e) => {
                    tracing::warn!(file = %path.display(), error = %e, "import: unreadable source")
                }
            }
        }
    }
    out
}

/// Resolve each `Component.param` of the sources to its definition and literal text.
fn resolve_sources(
    pkg_name: &str,
    sources: &[(String, String)],
) -> BTreeMap<String, (Source, String)> {
    use cadetta_syntax::ast::{ExprKind, MemberKind, UnOp};
    let mut out = BTreeMap::new();
    for (file, text) in sources {
        let parsed = cadetta_syntax::parser::parse(text);
        for component in parsed.file.components() {
            for member in &component.members {
                let (name, definition, cited, literal) = match &member.kind {
                    MemberKind::Param(p) => {
                        let mut e = &p.value;
                        if let ExprKind::Unary(UnOp::Neg, inner) = &e.kind {
                            e = inner;
                        }
                        let definition = match &e.kind {
                            ExprKind::Number(_) | ExprKind::Quantity(_) => Definition::Literal {
                                file: file.clone(),
                                start: p.value.span.start,
                                end: p.value.span.end,
                            },
                            ExprKind::Tolerance(..)
                            | ExprKind::Position(..)
                            | ExprKind::IsoClass(..) => Definition::Toleranced,
                            _ => Definition::Derived,
                        };
                        let literal = p.value.span.slice(text).to_owned();
                        (
                            p.name.name.clone(),
                            definition,
                            p.citation.is_some(),
                            literal,
                        )
                    }
                    MemberKind::Let(l) => (
                        l.name.name.clone(),
                        Definition::Derived,
                        false,
                        String::new(),
                    ),
                    _ => continue,
                };
                let path = format!("{pkg_name}::{}.{name}", component.name.name);
                out.insert(path, (Source { definition, cited }, literal));
            }
        }
    }
    out
}

/// `import ADAPTER FILE... --update [--write]`: pull CAD edits back into `.cdt` literals.
///
/// Without `--write` nothing on disk changes; with it the literals are rewritten and the lock's
/// `synced` values updated (adapters.md 7.4, config-cli.md 3).
// frob:ticket 01M4SWMCWHXKA8K6VG7Z1PNF3W
fn update_mode(
    project: &Project,
    scope: &[usize],
    parsed: &ImportArgs,
    globals: &Globals,
) -> Result<ImportReport, String> {
    let (pkg, adapter, read) = read_adapter(project, scope, parsed, globals)?;
    let id = adapter.id();
    let theirs = observed_values(&read.observed);
    let bindings_text =
        std::fs::read_to_string(pkg.dir.join(adapter.bindings_path())).unwrap_or_default();
    let set = BindingSet::load_str(id, &bindings_text).map_err(|e| e.to_string())?;
    let lock_path = project.root.join("cadetta.lock");
    let mut lock = Lock::load(&lock_path).map_err(|e| e.to_string())?;
    let bases: BTreeMap<(String, String), String> = lock
        .records()
        .filter_map(|r| match r {
            Record::SyncedValue(v) if v.adapter == id => {
                Some(((v.path.clone(), v.locator.clone()), v.value.clone()))
            }
            _ => None,
        })
        .collect();
    let sources = package_sources(pkg);
    let resolved = resolve_sources(&package_id(pkg).name, &sources);
    let mut entries = Vec::new();
    let mut defs = BTreeMap::new();
    for binding in set.entries() {
        let path = binding.path.to_string();
        for loc in &binding.locators {
            let Some((file, value)) = theirs.get(&loc.locator) else {
                tracing::debug!(%path, locator = %loc.locator, "update: not observed");
                continue;
            };
            let mut ours = value.clone();
            if let Some((src, literal)) = resolved.get(&path) {
                defs.insert(path.clone(), src.clone());
                if normalize(literal, "") != *value {
                    ours = literal.clone();
                }
            }
            entries.push(Entry {
                adapter: id.to_owned(),
                file: file.clone(),
                path: path.clone(),
                locator: loc.locator.clone(),
                base: bases.get(&(path.clone(), loc.locator.clone())).cloned(),
                ours,
                theirs: value.clone(),
                replaces: None,
                ours_tol: None,
                theirs_tol: None,
            });
        }
    }
    let plan = update::plan(&entries, &defs);
    let mut report = ImportReport {
        mode: "update".to_owned(),
        ..ImportReport::default()
    };
    let diff = update::render_diff(&plan);
    if !diff.is_empty() {
        report.items.push(diff);
    }
    for d in &read.diagnostics {
        report.notes.push(d.0.to_string());
    }
    if !parsed.write {
        tracing::info!(
            edits = plan.edits.len(),
            "import: update dry run, nothing written"
        );
        if !plan.edits.is_empty() {
            report
                .notes
                .push("nothing written; pass --write to apply".to_owned());
        }
        return Ok(report);
    }
    let mut rewritten = Vec::new();
    for (file, text) in &sources {
        if !plan.edits.iter().any(|e| e.file == *file) {
            continue;
        }
        rewritten.push((file, update::apply(text, file, &plan)?));
    }
    for (file, text) in rewritten {
        std::fs::write(pkg.dir.join(file), text).map_err(|e| e.to_string())?;
        tracing::info!(%file, "import: literals rewritten");
        report.written.push(file.clone());
    }
    for (adapter, path, locator, value) in &plan.lock_updates {
        let rec = Record::SyncedValue(SyncedValue {
            adapter: adapter.clone(),
            path: path.clone(),
            locator: locator.clone(),
            value: value.clone(),
        });
        lock.remove(&rec.key());
        lock.insert(rec);
    }
    if lock.save(&lock_path).map_err(|e| e.to_string())? {
        report.written.push("cadetta.lock".to_owned());
    }
    Ok(report)
}

fn rel(root: &FsPath, p: &FsPath) -> String {
    p.strip_prefix(root)
        .unwrap_or(p)
        .to_string_lossy()
        .into_owned()
}

/// Find the package whose `build/import/proposal.toml` exists.
fn proposal_package<'a>(project: &'a Project, scope: &[usize]) -> Option<&'a PackageDir> {
    scope
        .iter()
        .map(|i| &project.packages[*i])
        .find(|p| p.dir.join(PROPOSAL_DIR).join(PROPOSAL_TOML).is_file())
}

fn accept(project: &Project, scope: &[usize], parsed: &ImportArgs) -> Result<ImportReport, String> {
    let pkg = proposal_package(project, scope)
        .ok_or("no build/import/proposal.toml; run `cadetta import ADAPTER FILE...` first")?;
    let text = std::fs::read_to_string(pkg.dir.join(PROPOSAL_DIR).join(PROPOSAL_TOML))
        .map_err(|e| e.to_string())?;
    let proposal: Proposal = toml::from_str(&text).map_err(|e| format!("proposal.toml: {e}"))?;
    let adapter_id = parsed
        .adapter
        .clone()
        .unwrap_or_else(|| proposal.adapter.clone());
    let bindings_rel = pkg
        .config
        .adapters
        .iter()
        .find(|a| a.id() == adapter_id || a.name == adapter_id)
        .map_or_else(
            || format!("bindings/{adapter_id}.toml"),
            Adapter::bindings_path,
        );
    let bindings_file = pkg.dir.join(&bindings_rel);
    let existing_text = std::fs::read_to_string(&bindings_file).unwrap_or_default();
    let existing = BindingSet::load_str(&adapter_id, &existing_text).map_err(|e| e.to_string())?;
    let mut bound_paths: BTreeSet<String> = existing
        .entries()
        .iter()
        .map(|b| b.path.to_string())
        .collect();
    let mut bound_locators: BTreeSet<String> = existing
        .entries()
        .iter()
        .flat_map(|b| b.locators.iter().map(|l| l.locator.clone()))
        .collect();
    let pkg_name = package_id(pkg).name;
    let mut report = ImportReport {
        mode: "accept".to_owned(),
        ..ImportReport::default()
    };
    let planned = accept_plan(&proposal, &pkg_name, &mut bound_paths, &mut bound_locators);
    let appended = planned.appended;
    report.unchanged.extend(planned.unchanged);
    report.items.extend(planned.added);
    if !appended.is_empty() {
        if let Some(parent) = bindings_file.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let mut full = existing_text;
        if !full.is_empty() && !full.ends_with('\n') {
            full.push('\n');
        }
        if !full.is_empty() {
            full.push('\n');
        }
        full.push_str(&appended);
        std::fs::write(&bindings_file, full).map_err(|e| e.to_string())?;
        tracing::info!(file = %bindings_file.display(), "import: bindings written");
        report.written.push(bindings_rel);
    }
    let mut files: BTreeSet<&str> = proposal.items.iter().map(|i| i.file.as_str()).collect();
    files.retain(|f| {
        proposal
            .items
            .iter()
            .any(|i| i.file == *f && i.status != Status::Skip)
    });
    for file in files {
        let cdt_rel = component_file(file);
        let cdt = pkg.dir.join(&cdt_rel);
        if cdt.exists() {
            report.unchanged.push(cdt_rel);
            continue;
        }
        let items: Vec<&Item> = proposal
            .items
            .iter()
            .filter(|i| i.file == file && i.status != Status::Skip)
            .collect();
        let component = items[0].component.clone();
        if let Some(parent) = cdt.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(&cdt, render_model(&component, &items)).map_err(|e| e.to_string())?;
        tracing::info!(file = %cdt.display(), "import: component written");
        report.written.push(cdt_rel);
    }
    let lock_path = project.root.join("cadetta.lock");
    let mut lock = Lock::load(&lock_path).map_err(|e| e.to_string())?;
    let files: BTreeSet<&str> = proposal.items.iter().map(|i| i.file.as_str()).collect();
    for file in files {
        lock.insert(Record::Adopted(Adopted {
            adapter: adapter_id.clone(),
            file: file.to_owned(),
        }));
    }
    if lock.save(&lock_path).map_err(|e| e.to_string())? {
        report.written.push("cadetta.lock".to_owned());
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    /// A self-removing scratch directory (no extra dev-dependency).
    struct Dir(PathBuf);
    impl Dir {
        fn new() -> Self {
            static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
            let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let p = std::env::temp_dir().join(format!("cadetta-import-{}-{n}", std::process::id()));
            std::fs::create_dir_all(&p).unwrap();
            Self(p)
        }
    }
    impl Drop for Dir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    const ADAPTER: &str = r#"#!/usr/bin/env python3
import json, os, sys
here = os.path.dirname(os.path.abspath(__file__))
obs = json.load(open(os.path.join(here, "observed.json")))
info = {"name": "fake", "version": "0.1.0", "capabilities": ["read"],
        "native_units": "mm", "precision": 6, "platforms": ["linux"]}
for line in sys.stdin:
    req = json.loads(line)
    method = req["method"]
    if method == "initialize":
        res = info
    elif method == "read":
        res = {"observed": obs, "diagnostics": []}
    else:
        res = None
    sys.stdout.write(json.dumps({"jsonrpc": "2.0", "id": req["id"], "result": res}) + "\n")
    sys.stdout.flush()
    if method == "shutdown":
        break
"#;

    fn observed() -> Value {
        let member = |name: &str, nominal: &str| {
            serde_json::json!({
                "path": format!("acme.plate::Observed.{name}"),
                "value": {"nominal": {"r": nominal, "pi": 0}, "dim": {"m": 1},
                          "authored": "12", "exact": true, "unit_assumed": "mm"},
                "provenance": {"imported": {"adapter": "fake", "file": "cover_plate.FCStd",
                                             "native_name": name}}
            })
        };
        let binding = |name: &str, mode: &str| {
            serde_json::json!({
                "path": format!("acme.plate::Observed.{name}"), "adapter": "fake",
                "locators": [{"locator": format!("Sketch.{name}"), "kind": "dimension",
                              "mode": mode, "native_unit": "mm"}]
            })
        };
        serde_json::json!({
            "components": [{"members": [member("Bore_mm", "3/250"), member("Driven", "1/100")]}],
            "bindings": [binding("Bore_mm", "value"), binding("Driven", "ref")]
        })
    }

    /// Serializes script writes against adapter spawns: a forked child that
    /// inherits the still-open write handle makes exec fail with ETXTBSY.
    static SPAWN_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn project() -> Dir {
        let _guard = SPAWN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = Dir::new();
        std::fs::write(
            dir.0.join("cadetta.toml"),
            "[package]\nname = \"acme.plate\"\nversion = \"0.1.0\"\n[[adapter]]\nname = \"fake\"\ncommand = [\"./adapter.py\"]\n",
        )
        .unwrap();
        std::fs::create_dir_all(dir.0.join("src")).unwrap();
        std::fs::write(dir.0.join("src/lib.cdt"), "model A { x: Length = 1mm }\n").unwrap();
        std::fs::write(dir.0.join("observed.json"), observed().to_string()).unwrap();
        let script = dir.0.join("adapter.py");
        std::fs::write(&script, ADAPTER).unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
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
        let _guard = SPAWN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let code = run_import(&args, &globals, &mut out, &mut err);
        (
            code,
            String::from_utf8(out).unwrap(),
            String::from_utf8(err).unwrap(),
        )
    }

    // frob:tests crates/cadetta-cli/src/cmd/import.rs::run_import
    #[test]
    fn propose_writes_only_the_proposal() {
        let dir = project();
        let (code, out, e) = run(&dir, &["fake", "cover_plate.FCStd"]);
        assert_eq!((code, e.as_str()), (EXIT_OK, ""), "{out}");
        let toml = std::fs::read_to_string(dir.0.join("build/import/proposal.toml")).unwrap();
        assert!(toml.contains("CoverPlate"), "{toml}");
        assert!(toml.contains("Sketch.Bore_mm"), "{toml}");
        assert!(toml.contains("skip"), "driven dimension skipped: {toml}");
        let cdt = std::fs::read_to_string(dir.0.join("build/import/proposal.cdt")).unwrap();
        assert!(cdt.contains("bore"), "{cdt}");
        assert!(!dir.0.join("bindings").exists());
        assert!(!dir.0.join("cad/imported").exists());
        assert!(!dir.0.join("cadetta.lock").exists());
    }

    // frob:tests crates/cadetta-cli/src/cmd/import.rs::run_import
    #[test]
    fn accept_writes_bindings_cdt_and_lock_idempotently() {
        let dir = project();
        assert_eq!(run(&dir, &["fake", "cover_plate.FCStd"]).0, EXIT_OK);
        let (code, out, e) = run(&dir, &["--accept"]);
        assert_eq!((code, e.as_str()), (EXIT_OK, ""), "{out}");
        let bindings = std::fs::read_to_string(dir.0.join("bindings/fake.toml")).unwrap();
        assert!(
            bindings.contains("acme.plate::CoverPlate.bore"),
            "{bindings}"
        );
        assert!(!bindings.contains("driven"), "{bindings}");
        let set = BindingSet::load_str("fake", &bindings).unwrap();
        assert_eq!(set.entries().len(), 1);
        let cdt = std::fs::read_to_string(dir.0.join("cad/imported/CoverPlate.cdt")).unwrap();
        assert!(cdt.starts_with("model CoverPlate {"), "{cdt}");
        let lock = std::fs::read_to_string(dir.0.join("cadetta.lock")).unwrap();
        assert!(lock.contains("adopted fake cover_plate.FCStd"), "{lock}");

        let (code, out, _) = run(&dir, &["--accept"]);
        assert_eq!(code, EXIT_OK);
        assert!(out.contains("unchanged"), "{out}");
        assert_eq!(
            std::fs::read_to_string(dir.0.join("bindings/fake.toml")).unwrap(),
            bindings
        );
    }

    // frob:tests crates/cadetta-cli/src/cmd/import.rs::run_import
    #[test]
    fn never_exits_one() {
        let dir = project();
        assert_eq!(run(&dir, &[]).0, EXIT_USAGE);
        assert_eq!(run(&dir, &["fake"]).0, EXIT_USAGE);
        assert_eq!(run(&dir, &["nosuch", "f.FCStd"]).0, EXIT_USAGE);
        assert_eq!(run(&dir, &["--accept"]).0, EXIT_USAGE, "no proposal yet");
        assert_eq!(run(&dir, &["fake", "f", "--update"]).0, EXIT_OK);
    }

    // frob:tests crates/cadetta-cli/src/cmd/import.rs::decode_observed
    #[test]
    fn decodes_observed_and_applies_assume_unit() {
        let id = cadetta_ir::model::PackageId {
            name: "acme.plate".into(),
            version: "0.1.0".into(),
        };
        let projects = decode_observed(&observed(), "fake", id, Some("in")).unwrap();
        assert_eq!(projects.len(), 1);
        let proposal = propose("fake", &projects);
        assert_eq!(proposal.items.len(), 2);
        assert!(
            proposal
                .items
                .iter()
                .all(|i| i.unit.as_deref() == Some("in"))
        );
    }

    // frob:tests crates/cadetta-cli/src/cmd/import.rs::decode_observed
    #[test]
    fn decodes_frames_and_instances_into_the_builder() {
        let id = cadetta_ir::model::PackageId {
            name: "acme.plate".into(),
            version: "0.1.0".into(),
        };
        let mut obs = observed();
        let t = serde_json::json!([[1, 0, 0, 0], [0, 1, 0, 0], [0, 0, 1, 0]]);
        obs["frames"] =
            serde_json::json!([{"name": "F", "document": "cover_plate.FCStd", "transform": t}]);
        obs["instances"] = serde_json::json!([{"component": "c", "document": "cover_plate.FCStd", "transform": t}]);
        let projects = decode_observed(&obs, "fake", id, None).unwrap();
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].frames.len(), 1);
        assert_eq!(projects[0].instances.len(), 1);
    }

    const BORE: &str = "acme.plate::CoverPlate.bore";

    /// Propose and accept, then make CAD (12mm) differ from the adopted base and source (5mm).
    fn adopted_at_5mm(source: &str) -> Dir {
        let dir = project();
        assert_eq!(run(&dir, &["fake", "cover_plate.FCStd"]).0, EXIT_OK);
        assert_eq!(run(&dir, &["--accept"]).0, EXIT_OK);
        std::fs::write(
            dir.0.join("cad/imported/CoverPlate.cdt"),
            format!("model CoverPlate {{\n  bore = {source}\n}}\n"),
        )
        .unwrap();
        let lock_path = dir.0.join("cadetta.lock");
        let mut lock = Lock::load(&lock_path).unwrap();
        lock.insert(Record::SyncedValue(SyncedValue {
            adapter: "fake".into(),
            path: BORE.into(),
            locator: "Sketch.Bore_mm".into(),
            value: "5mm".into(),
        }));
        lock.save(&lock_path).unwrap();
        dir
    }

    fn synced(dir: &Dir) -> Option<String> {
        let lock = Lock::load(&dir.0.join("cadetta.lock")).unwrap();
        lock.records().find_map(|r| match r {
            Record::SyncedValue(v) if v.path == BORE => Some(v.value.clone()),
            _ => None,
        })
    }

    // frob:tests crates/cadetta-cli/src/cmd/import.rs::update_mode
    #[test]
    fn update_prints_diff_and_writes_nothing_without_write() {
        let dir = adopted_at_5mm("5mm");
        let cdt = dir.0.join("cad/imported/CoverPlate.cdt");
        let (src_before, lock_before) = (
            std::fs::read(&cdt).unwrap(),
            std::fs::read(dir.0.join("cadetta.lock")).unwrap(),
        );
        let (code, out, e) = run(&dir, &["fake", "cover_plate.FCStd", "--update"]);
        assert_eq!((code, e.as_str()), (EXIT_OK, ""), "{out}");
        assert!(out.contains("- 5mm\n+ 12mm"), "{out}");
        assert!(out.contains("CoverPlate.cdt"), "{out}");
        assert_eq!(std::fs::read(&cdt).unwrap(), src_before);
        assert_eq!(
            std::fs::read(dir.0.join("cadetta.lock")).unwrap(),
            lock_before
        );
        assert!(!out.contains("not implemented"), "{out}");
    }

    // frob:tests crates/cadetta-cli/src/cmd/import.rs::update_mode
    #[test]
    fn update_write_rewrites_literal_and_lock() {
        let dir = adopted_at_5mm("5mm");
        let (code, out, e) = run(&dir, &["fake", "cover_plate.FCStd", "--update", "--write"]);
        assert_eq!((code, e.as_str()), (EXIT_OK, ""), "{out}");
        let cdt = std::fs::read_to_string(dir.0.join("cad/imported/CoverPlate.cdt")).unwrap();
        assert_eq!(cdt, "model CoverPlate {\n  bore = 12mm\n}\n");
        assert_eq!(synced(&dir).as_deref(), Some("12mm"));
        assert!(out.contains("wrote cad/imported/CoverPlate.cdt"), "{out}");
        let (_, again, _) = run(&dir, &["fake", "cover_plate.FCStd", "--update"]);
        assert!(!again.contains("- "), "second run is a no-op: {again}");
    }

    // frob:tests crates/cadetta-cli/src/cmd/import.rs::update_mode
    #[test]
    fn update_write_never_edits_expressions() {
        let dir = adopted_at_5mm("2 * 2.5mm");
        let cdt = dir.0.join("cad/imported/CoverPlate.cdt");
        let before = std::fs::read(&cdt).unwrap();
        let (code, out, _) = run(&dir, &["fake", "cover_plate.FCStd", "--update", "--write"]);
        assert_eq!(code, EXIT_OK, "{out}");
        assert!(out.contains("derived"), "{out}");
        assert_eq!(std::fs::read(&cdt).unwrap(), before);
        assert_eq!(synced(&dir).as_deref(), Some("5mm"));
    }

    // frob:ticket 01M4SWN8G3QTQC5YK07VN1YQYC
    // frob:tests crates/cadetta-cli/src/cmd/import.rs::run_import
    #[test]
    fn json_validates_against_import_schema() {
        let schema: serde_json::Value =
            serde_json::from_str(include_str!("../../../../docs/schemas/import.json")).unwrap();
        let validator = jsonschema::validator_for(&schema).unwrap();
        let dir = project();
        let globals = Globals {
            format: Format::Json,
            cwd: Some(dir.0.clone()),
            config: None,
        };
        for args in [
            &["fake", "cover_plate.FCStd"][..],
            &["--accept"],
            &["fake", "f", "--update"],
        ] {
            let args: Vec<String> = args.iter().map(|a| (*a).to_owned()).collect();
            let (mut out, mut err) = (Vec::new(), Vec::new());
            let _guard = SPAWN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
            assert_eq!(run_import(&args, &globals, &mut out, &mut err), EXIT_OK);
            let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
            assert!(validator.is_valid(&v), "{v}");
        }
    }

    // frob:ticket 01M4YPDTD1977TMY4M2PPPNSDD
    // frob:tests crates/cadetta-cli/src/cmd/import.rs::read_adapter
    #[test]
    fn a_silent_adapter_fails_import_within_the_configured_timeout() {
        let dir = project();
        let toml = std::fs::read_to_string(dir.0.join("cadetta.toml")).unwrap();
        std::fs::write(dir.0.join("cadetta.toml"), format!("{toml}timeout = 1\n")).unwrap();
        {
            let _guard = SPAWN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
            let script = dir.0.join("adapter.py");
            std::fs::write(&script, "#!/bin/sh\nsleep 300\n").unwrap();
            std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        let start = std::time::Instant::now();
        let (code, _, e) = run(&dir, &["fake", "cover_plate.FCStd"]);
        assert_eq!(code, EXIT_USAGE, "{e}");
        assert!(e.contains("R008"), "{e}");
        assert!(start.elapsed() < std::time::Duration::from_secs(30));
    }
}
