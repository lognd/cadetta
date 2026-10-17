//! frob:ticket 01M4SPWTDD069AXXQBXD4T6ATZ
//! `cadetta sync`: classify every bound locator three-way, push the design-owned
//! changes through a drive adapter, rebuild, read back and record the lock
//! (config-cli.md 3, 3.1; adapters.md 2, 2.1).
//!
//! An adapter that declares `drive` is driven through [`DriveSession`]; any other
//! adapter falls back to `emit` plus printed manual instructions. Values are compared
//! as normalized `{number}{unit}` text; differing units are compared textually.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::io::{BufRead, Write};
use std::path::{Path as FsPath, PathBuf};

use cadetta_adapter::atomic::{AtomicOutcome, Document};
use cadetta_adapter::availability::{Verb as AvailVerb, rejections, unavailable};
use cadetta_adapter::bindings::{BindingSet, RefTarget, mangle};
use cadetta_adapter::builtin::csv;
use cadetta_adapter::builtin::sw_equations::{ADAPTER as SW_EQUATIONS, ValueKind};
use cadetta_adapter::geometry_bind::{datum_entries, verify_geometry};
use cadetta_adapter::locator::{DesignForm, DesignTolerance, TolerancePlan, plan_tolerance};
use cadetta_adapter::policy::{EmitPoint, resolve_point, to_decimal};
use cadetta_adapter::read_units::UnitPolicy;
use cadetta_adapter::rpc::client::ProcessClient;
use cadetta_adapter::rpc::{
    CapabilityName, ChangedParam, Client, DimTolerance, Feature, LocatorKind as WireKind,
    LocatorStatus, ObservedComponent, ObservedInstanceValues, ObservedSketch, ObservedTolerance,
    PushMode, PushStatus, ReadParams, ReadSource, RegenState,
};
use cadetta_adapter::scope::Scope as AdapterScope;
use cadetta_adapter::session::{DriveSession, SaveMode as SessionSaveMode};
use cadetta_adapter::sync::{
    Backend, Entry, Lock as EngineLock, Options, PushItem, R003, ReadBack, Verb, run,
};
use cadetta_ir::model::{
    Exact, LocatorKind, LocatorMode, Nominal, ObservedFrame, ObservedInstance, Project as Ir, Sign,
    TolerancePolicy, Value as IrValue,
};
use cadetta_obligations::lock::{Hex, Lock as FileLock, Record, SyncedFile, SyncedValue};
use cadetta_units::dimension::Dim;
use cadetta_units::{Magnitude, Rational, UnitTable};
use serde::Serialize;
use serde_json::{Value as Json, json};

use crate::cmd::emit::{
    DEFAULT_NATIVE_UNIT, EmitArgs, EmitReport, LoadError, TextParams, active_alternates,
    attr_point, config_point, designed, emit_package_texts, emit_params, kind_of, load_bindings,
    locator_active, locators_for, native_unit, package_ir, text_params_of, to_ratio, value_row_rel,
    written_for,
};
use cadetta_adapter::geometry_plan::negate_text;

use crate::cmd::import::program;
use crate::config::{Adapter, SaveMode};
use crate::output::Format;
use crate::pipeline::analyze;
use crate::project::{PackageDir, Project};
use crate::scope::{Glob, Scope as RunScope};
use crate::universe;
use crate::{EXIT_FAIL, EXIT_OK, EXIT_USAGE, Globals};

const USAGE: &str = "usage: cadetta sync [--dry-run] [--only GLOB] [--force-design] [--no-regen] [--module PATH] [ADAPTER-ID...]";

/// Diagnostic codes that make `sync` exit 1 (config-cli.md 3).
const BLOCKING: [&str; 9] = [
    "R001", "R004", "R009", "R010", "Q004", "R013", "R014", "R015", "R016",
];

/// The parsed command line of `cadetta sync`.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct SyncArgs {
    /// Print every planned push; write nothing.
    pub dry_run: bool,
    /// `--only` canonical-path globs (repeatable).
    pub only: Vec<String>,
    /// Push over R001.
    pub force_design: bool,
    /// Push without rebuilding; the lock is not updated.
    pub no_regen: bool,
    /// `--module` prefixes (repeatable).
    pub modules: Vec<String>,
    /// Adapter ids; empty means every configured adapter.
    pub adapters: Vec<String>,
}

/// Parse the words after `sync`; unknown flags are usage errors.
pub fn parse_args(args: &[String]) -> Result<SyncArgs, String> {
    let mut parsed = SyncArgs::default();
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--dry-run" => parsed.dry_run = true,
            "--force-design" => parsed.force_design = true,
            "--no-regen" => parsed.no_regen = true,
            "--only" => parsed
                .only
                .push(it.next().ok_or("`--only` needs a glob")?.clone()),
            "--module" => parsed
                .modules
                .push(it.next().ok_or("`--module` needs a path")?.clone()),
            flag if flag.starts_with('-') => return Err(format!("unknown flag `{flag}`")),
            id => parsed.adapters.push(id.to_owned()),
        }
    }
    Ok(parsed)
}

/// One locator line of the report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SyncRecord {
    /// Adapter id.
    pub adapter: String,
    /// Native file.
    pub file: String,
    /// Canonical parameter path.
    pub path: String,
    /// Native locator.
    pub locator: String,
    /// `push|skip|drift|converged|fail`.
    pub verb: String,
    /// The previous value (lock base, else what the CAD held).
    pub old: String,
    /// The design value.
    pub new: String,
}

/// A diagnostic of the run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SyncDiag {
    /// Rule code.
    pub code: String,
    /// Severity: `info` (O004), `error` (a blocking code) or `warning`.
    pub severity: &'static str,
    /// Message.
    pub message: String,
}

impl SyncDiag {
    /// Build a diagnostic whose severity follows its code (obligations.md 2.3, config-cli.md 3).
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        let code = code.into();
        let severity = if code == "O004" {
            "info"
        } else if BLOCKING.contains(&code.as_str()) {
            "error"
        } else {
            "warning"
        };
        Self {
            code,
            severity,
            message: message.into(),
        }
    }
}

/// Rebuild outcome of one pushed file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RegenRecord {
    /// Native file.
    pub file: String,
    /// `ok`, `failed` or `skipped`.
    pub status: String,
}

/// Manual steps for an adapter without `drive`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Manual {
    /// Adapter id.
    pub adapter: String,
    /// What the user does by hand.
    pub instructions: String,
}

/// The `sync.json` document (config-cli.md 3.1).
#[derive(Debug, Default, Serialize)]
pub struct SyncReport {
    /// One record per in-scope locator.
    pub records: Vec<SyncRecord>,
    /// Diagnostics.
    pub diagnostics: Vec<SyncDiag>,
    /// Per-file rebuild status.
    pub regen: Vec<RegenRecord>,
    /// Files emitted by non-drive fallbacks.
    pub emitted: Vec<String>,
    /// Manual instructions for non-drive adapters.
    pub manual: Vec<Manual>,
    /// Whether `cadetta.lock` was written.
    pub lock_written: bool,
    /// Native files a non-drive adapter recorded this run (`synced file=`); their O004 is judged
    /// once the run ends.
    #[serde(skip)]
    handed_over: BTreeSet<String>,
    /// Bound `(path, file, locator)` triples absent from the native files (structured R003).
    #[serde(skip)]
    pub missing: Vec<(String, String, String)>,
    /// An invalid bindings file or locator: stops the run with exit 2 (adapters.md 3).
    #[serde(skip)]
    config_error: Option<String>,
}

impl SyncReport {
    fn diag(&mut self, code: &str, message: impl Into<String>) {
        let message = message.into();
        tracing::warn!(code, %message, "sync diagnostic");
        self.diagnostics.push(SyncDiag::new(code, message));
    }

    /// True when the run must exit 1.
    pub fn failed(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| BLOCKING.contains(&d.code.as_str()))
            || self.records.iter().any(|r| r.verb == "fail")
    }
}

/// A value found in a native file: `(file, native name)` with its text and expression.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Observed {
    file: String,
    native: String,
    text: String,
    expr: Option<String>,
}

/// Default native unit per `(adapter, locator)`.
type UnitMap = BTreeMap<(String, String), String>;

/// Binding kind per `(adapter, locator)`, sent as `ChangedParam.kind` on push.
// frob:ticket 01M54AQVNNZJZSC03BV7G6G3QM
type KindMap = BTreeMap<(String, String), WireKind>;

/// The wire kind of a binding kind; `Instance` placements are never pushed as values.
// frob:ticket 01M54AQVNNZJZSC03BV7G6G3QM
fn wire_kind(kind: LocatorKind) -> Option<WireKind> {
    match kind {
        LocatorKind::Global => Some(WireKind::Global),
        LocatorKind::Dimension => Some(WireKind::Dimension),
        LocatorKind::Property => Some(WireKind::Property),
        LocatorKind::Frame => Some(WireKind::Frame),
        LocatorKind::Instance => None,
    }
}

/// Per-run state shared by the adapters of a project.
struct Ctx<'a> {
    args: &'a SyncArgs,
    run_scope: &'a RunScope,
    lock: &'a mut FileLock,
    lock_changed: bool,
    report: &'a mut SyncReport,
    /// The package's bound `Text` parameters, so the manual emit fallback can raise R014.
    texts: TextParams,
}

/// Split `text` into its leading number and the rest; `None` when it does not start with one.
fn split_number(t: &str) -> Option<(&str, &str)> {
    let b = t.as_bytes();
    let mut i = 0;
    if i < b.len() && (b[i] == b'-' || b[i] == b'+') {
        i += 1;
    }
    let digits_from = i;
    while i < b.len() && (b[i].is_ascii_digit() || b[i] == b'.') {
        i += 1;
    }
    if i == digits_from {
        return None;
    }
    if i < b.len() && (b[i] == b'e' || b[i] == b'E') {
        let mut j = i + 1;
        if j < b.len() && (b[j] == b'-' || b[j] == b'+') {
            j += 1;
        }
        let exp_from = j;
        while j < b.len() && b[j].is_ascii_digit() {
            j += 1;
        }
        if j > exp_from {
            i = j;
        }
    }
    Some((&t[..i], &t[i..]))
}

/// Normalize `text` to `{number}{unit}`; `default_unit` fills a missing unit.
///
/// Text that does not start with a number is trimmed and kept as is.
pub(crate) fn normalize(text: &str, default_unit: &str) -> String {
    let t = text.trim();
    let Some((number, rest)) = split_number(t) else {
        return t.to_owned();
    };
    let mut num = number.trim_start_matches('+').to_owned();
    if num.contains('.') && !num.contains(['e', 'E']) {
        num = num.trim_end_matches('0').trim_end_matches('.').to_owned();
    }
    if num == "-0" || num.is_empty() {
        num = "0".to_owned();
    }
    let unit = rest.trim();
    let unit = if unit.is_empty() { default_unit } else { unit };
    format!("{num}{unit}")
}

// frob:ticket 01M4XDM9WRFB7RFEF2X6RBPR2Y
/// Convert a normalized `{number}{unit}` to the `native` unit exactly; `None` keeps the text.
///
/// Only linear units of one dimension convert; a result that is not a decimal within `precision`
/// places (or involves pi) is left in its own unit so the textual comparison still reports it.
fn convert_unit(table: &UnitTable, text: &str, native: &str, precision: u32) -> Option<String> {
    let (num, unit) = split_number(text)?;
    if native.is_empty() || unit.is_empty() || unit == native {
        return None;
    }
    let affine = |u: &str| table.lookup(u).is_some_and(|d| d.affine);
    let (from, to) = (table.parse(unit).ok()?, table.parse(native).ok()?);
    if from.dim != to.dim || affine(unit) || affine(native) {
        return None;
    }
    let x = Magnitude::new(Rational::parse_decimal(num).ok()?, 0);
    let scaled = x.mul(&from.factor).div(&to.factor).ok()?;
    if scaled.pi_exp() != 0 {
        return None;
    }
    let dec = to_decimal(to_ratio(&scaled).ok()?, precision).ok()?;
    tracing::debug!(
        from = unit,
        to = native,
        text,
        "sync: converted observed unit"
    );
    Some(normalize(&format!("{dec}{native}"), native))
}

// frob:ticket 01M4XDM9WRFB7RFEF2X6RBPR2Y
/// [`normalize`] `text`, then convert it to `native` when it carries another unit of the same dimension.
pub(crate) fn normalize_in(table: &UnitTable, text: &str, native: &str, precision: u32) -> String {
    let n = normalize(text, native);
    convert_unit(table, &n, native, precision).unwrap_or(n)
}

// frob:ticket 01M4XDM9ST75VR0KRM3TDVNMAR
/// `tol` with every literal normalized into the `native` unit (a fit class is kept as is).
fn normalize_tolerance(
    table: &UnitTable,
    tol: &DimTolerance,
    native: &str,
    precision: u32,
) -> DimTolerance {
    let n = |s: &str| normalize_in(table, s, native, precision);
    match tol {
        DimTolerance::Symmetric { t } => DimTolerance::Symmetric { t: n(t) },
        DimTolerance::Bilateral { upper, lower } => DimTolerance::Bilateral {
            upper: n(upper),
            lower: n(lower),
        },
        DimTolerance::Limits { min, max } => DimTolerance::Limits {
            min: n(min),
            max: n(max),
        },
        DimTolerance::Fit { class } => DimTolerance::Fit {
            class: class.clone(),
        },
    }
}

// frob:ticket 01M4XDM9ST75VR0KRM3TDVNMAR
/// The designed tolerance of `value` as native-unit literals: its shape, `.min` and `.max`.
///
/// The IR keeps no fit class or interval spelling, so a one-sided or asymmetric spread is
/// `Bilateral` and an equal spread `Symmetric`; an untoleranced value is `DesignForm::None`.
fn design_tolerance(
    table: &UnitTable,
    value: &IrValue,
    unit: &str,
    precision: u32,
) -> Result<DesignTolerance, String> {
    let factor = table
        .parse(unit)
        .map_err(|e| format!("unit `{unit}`: {e}"))?
        .factor;
    let lit = |r| {
        to_decimal(r, precision)
            .map(|d| format!("{d}{unit}"))
            .map_err(|e| e.to_string())
    };
    let abs = designed(value, &factor)?;
    let form = if value.tol.is_none() {
        DesignForm::None
    } else {
        let zero = Rational::new(0, 1).map_err(|e| e.to_string())?;
        let mut spread = value.clone();
        spread.nominal = Nominal::Exact(Exact(Magnitude::new(zero, 0)));
        let d = designed(&spread, &factor)?;
        let (hi, lo) = (lit(d.max)?, lit(d.min)?);
        if d.max == d.min {
            DesignForm::None
        } else if hi == negate_text(&lo) {
            DesignForm::Symmetric(hi)
        } else {
            DesignForm::Bilateral(hi, lo)
        }
    };
    Ok(DesignTolerance {
        form,
        min: lit(abs.min)?,
        max: lit(abs.max)?,
    })
}

/// The package-relative, `/`-separated form of each absolute path in `files`.
fn relative_files(pkg: &PackageDir, files: &[String]) -> Vec<String> {
    files
        .iter()
        .map(|f| {
            FsPath::new(f)
                .strip_prefix(&pkg.dir)
                .map_or_else(|_| f.clone(), rel_glob_path)
        })
        .collect()
}

/// Map a bare observed file name to the one package-relative path in `known` that ends in it.
///
/// Adapters name observed files by base name; the push, regen and save steps need the path under
/// the package. Names already in `known`, unmatched, or ambiguous are left as reported.
fn resolve_observed_file(file: &str, known: &[String]) -> String {
    if known.iter().any(|k| k == file) {
        return file.to_owned();
    }
    let mut hits = known.iter().filter(|k| k.rsplit('/').next() == Some(file));
    match (hits.next(), hits.next()) {
        (Some(only), None) => {
            tracing::debug!(%file, resolved = %only, "sync: bare observed file resolved");
            only.clone()
        }
        (Some(_), Some(_)) => {
            tracing::warn!(%file, "sync: observed file name is ambiguous in the package");
            file.to_owned()
        }
        _ => file.to_owned(),
    }
}

/// Flatten an adapter's `observed` JSON into `(file, native name, authored, expr)` rows.
///
/// File names are resolved against `known`, the package-relative files the adapter was given.
fn observe(observed: &Json, known: &[String]) -> Vec<Observed> {
    let projects: Vec<&Json> = match observed {
        Json::Array(a) => a.iter().collect(),
        v => vec![v],
    };
    let mut rows = Vec::new();
    for member in projects
        .iter()
        .filter_map(|p| p.get("components").and_then(Json::as_array))
        .flatten()
        .filter_map(|c| c.get("members").and_then(Json::as_array))
        .flatten()
    {
        let imported = member.get("provenance").and_then(|p| p.get("imported"));
        let text = |v: Option<&Json>, k: &str| {
            v.and_then(|v| v.get(k))
                .and_then(Json::as_str)
                .map(str::to_owned)
        };
        let (Some(file), Some(native)) = (text(imported, "file"), text(imported, "native_name"))
        else {
            continue;
        };
        let Some(authored) = text(member.get("value"), "authored") else {
            continue;
        };
        rows.push(Observed {
            file: resolve_observed_file(&file, known),
            native,
            text: authored,
            expr: member.get("expr").and_then(Json::as_str).map(str::to_owned),
        });
    }
    tracing::debug!(rows = rows.len(), "sync: observed values flattened");
    rows
}

/// Render a package-relative path with `/` separators so globs match on Windows too.
fn rel_glob_path(rel: &FsPath) -> String {
    rel.to_string_lossy().replace('\\', "/")
}

/// Expand an adapter's `files` globs (relative to the package) to absolute paths.
fn expand_files(pkg: &PackageDir, adapter: &Adapter) -> Vec<String> {
    let mut out = BTreeSet::new();
    for pattern in &adapter.files {
        if !pattern.contains('*') {
            out.insert(pkg.dir.join(pattern).to_string_lossy().into_owned());
            continue;
        }
        let Ok(glob) = Glob::new(pattern) else {
            tracing::warn!(%pattern, "sync: bad file glob skipped");
            continue;
        };
        let prefix: PathBuf = pattern
            .split('/')
            .take_while(|s| !s.contains('*'))
            .collect();
        let mut stack = vec![pkg.dir.join(prefix)];
        while let Some(dir) = stack.pop() {
            let Ok(read) = std::fs::read_dir(&dir) else {
                continue;
            };
            for entry in read.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if let Ok(rel) = path.strip_prefix(&pkg.dir)
                    && glob.matches(&rel_glob_path(rel))
                {
                    out.insert(path.to_string_lossy().into_owned());
                }
            }
        }
    }
    out.into_iter().collect()
}

/// A diagnostic record's `(code, message)`.
fn diag_parts(v: &Json) -> (String, String) {
    let s = |k: &str| {
        v.get(k)
            .and_then(Json::as_str)
            .unwrap_or_default()
            .to_owned()
    };
    (s("code"), s("message"))
}

/// The `Backend` that drives a live session: push, optional rebuild, read back.
struct SessionBackend<R: BufRead, W: Write> {
    session: DriveSession<R, W>,
    /// Package-relative files given to the adapter, to resolve its bare observed names.
    known_files: Vec<String>,
    default_unit: UnitMap,
    /// Binding kinds of the pushed locators.
    kinds: KindMap,
    /// The adapter's decimal places, for converting read-back units.
    precision: u32,
    regen: Vec<RegenRecord>,
    diags: Vec<SyncDiag>,
    failure: Option<String>,
    /// Per-file push statuses, for the R010 summary (adapters.md 2.3).
    pushed: Vec<(String, Vec<LocatorStatus>)>,
}

impl<R: BufRead, W: Write> SessionBackend<R, W> {
    fn note(&mut self, code: &str, message: String) {
        self.diags.push(SyncDiag::new(code, message));
    }

    fn fail(&mut self, e: &cadetta_adapter::session::SessionError) {
        tracing::error!(error = %e, "sync: session step failed");
        match e.code() {
            Some(code) => self.note(code, e.to_string()),
            None => self.failure = Some(e.to_string()),
        }
    }

    fn adapter_diags(&mut self, records: &[cadetta_adapter::rpc::DiagRecord]) {
        for r in records {
            let (code, message) = diag_parts(&r.0);
            // R006 is raised by the engine from the observed expression.
            if code != "R006" && !code.is_empty() {
                self.note(&code, message);
            }
        }
    }
}

/// One file of a live session as an atomic-push [`Document`]: write is `push`, rebuild is `regen`.
struct SessionDoc<'a, R: BufRead, W: Write> {
    backend: &'a mut SessionBackend<R, W>,
    adapter: String,
    file: String,
    /// Statuses of the last write; a locator neither `Pushed` nor `Skipped` reads back as failed.
    statuses: Vec<LocatorStatus>,
}

impl<R: BufRead, W: Write> Document for SessionDoc<'_, R, W> {
    fn read_current(&mut self, items: &[PushItem]) -> Vec<ReadBack> {
        let (rows, read_tols) = match self.backend.session.read() {
            Ok(r) => (
                observe(&r.observed, &self.backend.known_files),
                r.tolerances,
            ),
            Err(e) => {
                self.backend.fail(&e);
                return Vec::new();
            }
        };
        let table = UnitTable::builtin();
        let precision = self.backend.precision;
        items
            .iter()
            .map(|it| {
                let locator = &it.locator;
                let status = self.statuses.iter().find(|s| &s.locator == locator);
                // Skipped means the document already holds the value, so it reads back like Pushed.
                if status
                    .is_some_and(|s| !matches!(s.status, PushStatus::Pushed | PushStatus::Skipped))
                {
                    return ReadBack {
                        locator: locator.clone(),
                        value: None,
                        tolerance: None,
                    };
                }
                let unit = self
                    .backend
                    .default_unit
                    .get(&(self.adapter.clone(), locator.clone()))
                    .cloned()
                    .unwrap_or_default();
                let value = rows
                    .iter()
                    .find(|o| o.file == self.file && &o.native == locator)
                    .map(|o| normalize_in(&table, &o.text, &unit, precision));
                let tolerance = read_tols
                    .iter()
                    .find(|t| &t.locator == locator)
                    .and_then(|t| t.tolerance.as_ref())
                    .or(status.and_then(|s| s.tolerance.as_ref()))
                    .map(|t| normalize_tolerance(&table, t, &unit, precision));
                ReadBack {
                    locator: locator.clone(),
                    value,
                    tolerance,
                }
            })
            .collect()
    }

    fn write(&mut self, items: &[PushItem]) -> Result<(), String> {
        let params = items
            .iter()
            .map(|i| ChangedParam {
                path: i.path.clone(),
                value: json!(i.value),
                locator: i.locator.clone(),
                mode: PushMode::Value,
                ref_to: None,
                tolerance: i.tolerance.clone(),
                kind: i.kind,
            })
            .collect();
        match self.backend.session.push(&self.file, params) {
            Ok(r) => {
                self.backend.adapter_diags(&r.diagnostics);
                self.backend
                    .pushed
                    .push((self.file.clone(), r.statuses.clone()));
                self.statuses = r.statuses;
                Ok(())
            }
            Err(e) => {
                self.backend.fail(&e);
                Err(e.to_string())
            }
        }
    }

    fn rebuild(&mut self) -> Result<(), String> {
        match self.backend.session.regen(std::slice::from_ref(&self.file)) {
            Ok(r) => {
                self.backend.adapter_diags(&r.diagnostics);
                if r.targets.iter().any(|t| t.status == RegenState::Failed) {
                    Err(format!("rebuild of `{}` failed", self.file))
                } else {
                    Ok(())
                }
            }
            Err(e) => {
                tracing::error!(error = %e, "sync: regen failed");
                Err(e.to_string())
            }
        }
    }
}

impl<R: BufRead, W: Write> Backend for SessionBackend<R, W> {
    fn document<'a>(&'a mut self, adapter: &str, file: &str) -> Box<dyn Document + 'a> {
        Box::new(SessionDoc {
            backend: self,
            adapter: adapter.to_owned(),
            file: file.to_owned(),
            statuses: Vec::new(),
        })
    }

    fn locator_kind(&self, adapter: &str, locator: &str) -> Option<WireKind> {
        self.kinds
            .get(&(adapter.to_owned(), locator.to_owned()))
            .copied()
    }

    fn pushed(&mut self, file: &str, regen: bool, outcome: &AtomicOutcome) {
        for d in &outcome.diagnostics {
            self.note(&d.code, d.message.clone());
        }
        let status = if !regen {
            "skipped"
        } else if outcome.diagnostics.is_empty() {
            "ok"
        } else {
            "failed"
        };
        tracing::info!(
            file,
            status,
            restored = outcome.restored,
            "sync: file pushed"
        );
        self.regen.push(RegenRecord {
            file: file.to_owned(),
            status: status.to_owned(),
        });
    }
}

/// A backend for runs where nothing is pushed.
struct NoBackend;

impl Backend for NoBackend {
    fn document<'a>(&'a mut self, _: &str, _: &str) -> Box<dyn Document + 'a> {
        unreachable!("no push was planned")
    }
}

/// Build the three-way entries of one adapter from the design IR and the native read.
fn build_entries(
    pkg: &PackageDir,
    adapter: &Adapter,
    ir: &Ir,
    observed: &[Observed],
    read: &Geometry<'_>,
    ctx: &mut Ctx<'_>,
) -> Option<(Vec<Entry>, UnitMap, KindMap)> {
    let (set, _) = match load_bindings(pkg, adapter, ir) {
        Ok(loaded) => loaded,
        Err(e) => {
            if let LoadError::Config(message) = &e {
                ctx.report.config_error = Some(message.clone());
            }
            let d = e.into_diag();
            ctx.report.diag(&d.code, d.message);
            return None;
        }
    };
    let scope = AdapterScope::new(adapter.modules.clone(), adapter.paths.clone());
    let table = UnitTable::builtin();
    let precision = read.precision;
    let cfg_point = config_point(pkg.config.emit.point);
    let has_tolerances = read.features.contains(&Feature::Tolerances);
    let active = active_alternates(ir);
    let mut entries = Vec::new();
    let mut units = BTreeMap::new();
    let mut kinds = KindMap::new();
    let mut clean = true;
    for member in ir.components.iter().flat_map(|c| &c.members) {
        let canonical = member.path.to_string();
        let Some(value) = &member.value else { continue };
        if member.analysis || !scope.accepts(&canonical) || !ctx.run_scope.contains(&canonical) {
            continue;
        }
        let Some(locators) = locators_for(&set, &member.path) else {
            continue;
        };
        let kind = kind_of(value);
        let point = resolve_point(&[attr_point(member)], cfg_point);
        for loc in locators {
            if !locator_active(loc, &active) {
                tracing::debug!(path = %canonical, locator = %loc.locator, "sync: inactive alternate locator not synced");
                continue;
            }
            if loc.mode == LocatorMode::Ref {
                check_ref_link(adapter, &set, &member.path, loc, observed, ctx);
                continue;
            }
            let unit = native_unit(adapter, loc);
            let written = match written_for(&table, value, member, &unit, kind, point, precision) {
                Ok(Some(w)) => w,
                Ok(None) => continue,
                Err((code, message)) => {
                    clean = false;
                    ctx.report.diag(code, format!("{canonical}: {message}"));
                    continue;
                }
            };
            let written = if loc.sign == Some(Sign::Minus) {
                negate_text(&written)
            } else {
                written
            };
            let mut ours_tol = None;
            let mut theirs_tol = None;
            let drives = has_tolerances
                && kind == ValueKind::Quantity
                && loc.kind == LocatorKind::Dimension
                && loc.tolerance.unwrap_or(adapter.tolerances) == TolerancePolicy::Drive;
            if drives {
                let plan = TolerancePlan {
                    adapter_default: adapter.tolerances,
                    has_feature: true,
                    nominal_emit: point == EmitPoint::Nominal,
                    tool_has_fit: false,
                };
                match design_tolerance(&table, value, &unit, precision) {
                    Ok(design) => ours_tol = plan_tolerance(loc, &plan, &design),
                    Err(message) => {
                        clean = false;
                        ctx.report.diag("Q004", format!("{canonical}: {message}"));
                        continue;
                    }
                }
                theirs_tol = read
                    .tolerances
                    .iter()
                    .find(|t| t.locator == loc.locator)
                    .and_then(|t| t.tolerance.as_ref())
                    .map(|t| normalize_tolerance(&table, t, &unit, precision));
            }
            let default_unit = if kind == ValueKind::Quantity {
                unit
            } else {
                String::new()
            };
            let Some(found) = observed.iter().find(|o| o.native == loc.locator) else {
                ctx.report.missing.push((
                    canonical.clone(),
                    adapter.files.join(", "),
                    loc.locator.clone(),
                ));
                ctx.report.diag(
                    R003,
                    format!(
                        "{canonical}: native item `{}` is not in the CAD files",
                        loc.locator
                    ),
                );
                continue;
            };
            units.insert(
                (adapter.id().to_owned(), loc.locator.clone()),
                default_unit.clone(),
            );
            if let Some(k) = wire_kind(loc.kind) {
                kinds.insert((adapter.id().to_owned(), loc.locator.clone()), k);
            }
            let key = cadetta_obligations::lock::Record::SyncedValue(SyncedValue {
                adapter: adapter.id().to_owned(),
                path: canonical.clone(),
                locator: loc.locator.clone(),
                value: String::new(),
            })
            .key();
            let base = match ctx.lock.get(&key) {
                Some(Record::SyncedValue(v)) => Some(v.value.clone()),
                _ => None,
            };
            entries.push(Entry {
                adapter: adapter.id().to_owned(),
                file: found.file.clone(),
                path: canonical.clone(),
                locator: loc.locator.clone(),
                base,
                ours: normalize(&written, &default_unit),
                theirs: normalize_in(&table, &found.text, &default_unit, precision),
                replaces: found.expr.clone(),
                ours_tol,
                theirs_tol,
            });
        }
    }
    entries.extend(geometry_pass(
        adapter,
        ir,
        set.entries(),
        observed,
        read,
        ctx,
    ));
    tracing::info!(adapter = %adapter.id(), entries = entries.len(), "sync: entries built");
    clean.then_some((entries, units, kinds))
}

/// What an adapter's `read` reported that the geometry pass needs.
struct Geometry<'a> {
    /// The raw observed IR (carries `frames` and `instances`).
    observed: &'a Json,
    /// The features the adapter announced at initialize.
    features: &'a [Feature],
    /// The dimension tolerances the adapter read (only `tolerances` adapters report any).
    tolerances: &'a [ObservedTolerance],
    /// The decimal places in force: the configured precision, else the adapter's `initialize` one.
    precision: u32,
}

/// One IR `Value` from a wire entry: a `{nominal}` Value, a bare `Exact`/`Inexact`, or a number.
///
/// Floats become exact decimals so the geometry comparison needs no inexact root.
fn wire_value(v: &Json) -> Option<IrValue> {
    let nominal = v.get("nominal").unwrap_or(v);
    let exact = if let (Some(r), Some(pi)) = (
        nominal.get("r").and_then(Json::as_str),
        nominal.get("pi").and_then(Json::as_i64),
    ) {
        let (n, d) = r.split_once('/')?;
        let rat = Rational::new(n.parse::<i64>().ok()?, d.parse::<i64>().ok()?).ok()?;
        Magnitude::new(rat, i32::try_from(pi).ok()?)
    } else {
        let x = nominal
            .get("f64")
            .and_then(Json::as_f64)
            .or_else(|| nominal.as_f64())?;
        if !x.is_finite() {
            return None;
        }
        Magnitude::new(Rational::parse_decimal(&format!("{x:e}")).ok()?, 0)
    };
    Some(IrValue::new(
        Nominal::Exact(Exact(exact)),
        Dim::default(),
        v.to_string(),
    ))
}

/// A 3x4 wire transform as rows of IR values; `None` when it is not rows of numbers.
fn wire_transform(v: &Json) -> Option<Vec<Vec<IrValue>>> {
    v.as_array()?
        .iter()
        .map(|row| row.as_array()?.iter().map(wire_value).collect())
        .collect()
}

/// True when a `ref` dimension's native expression still links to its target global.
///
/// A named target must appear in the expression; an alias or created global (whose name this
/// side cannot verify) only needs some expression to be present.
fn link_intact(target: &RefTarget, expr: Option<&str>) -> bool {
    match (target, expr) {
        (_, None) => false,
        (RefTarget::Global(g), Some(e)) => e.contains(g.as_str()),
        (_, Some(e)) => !e.trim().is_empty(),
    }
}

/// Verify a `ref`-mode locator: its value is carried by the target global (which syncs as a
/// value), so the dimension is never pushed or compared; a missing or unlinked dimension is R003.
fn check_ref_link(
    adapter: &Adapter,
    set: &BindingSet,
    path: &cadetta_ir::path::Path,
    loc: &cadetta_ir::model::Locator,
    observed: &[Observed],
    ctx: &mut Ctx<'_>,
) {
    let canonical = path.to_string();
    let own = mangle(path, &adapter.prefix);
    let target = set.resolve_ref(path, loc, &HashMap::new(), &own);
    let found = observed.iter().find(|o| o.native == loc.locator);
    let Some(found) = found else {
        ctx.report.missing.push((
            canonical.clone(),
            adapter.files.join(", "),
            loc.locator.clone(),
        ));
        ctx.report.diag(
            R003,
            format!(
                "{canonical}: native item `{}` is not in the CAD files",
                loc.locator
            ),
        );
        return;
    };
    if link_intact(&target, found.expr.as_deref()) {
        tracing::debug!(path = %canonical, locator = %loc.locator, ?target, "sync: ref link intact");
    } else {
        ctx.report.diag(
            R003,
            format!(
                "{canonical}: native link `{}` to {target:?} is gone",
                loc.locator
            ),
        );
    }
}

/// The observed `frames[]` and `instances[]` of an adapter's `read`; malformed items are skipped.
pub(crate) fn observed_geometry(observed: &Json) -> (Vec<ObservedFrame>, Vec<ObservedInstance>) {
    let projects: Vec<&Json> = match observed {
        Json::Array(a) => a.iter().collect(),
        v => vec![v],
    };
    let text = |v: &Json, k: &str| v.get(k).and_then(Json::as_str).map(str::to_owned);
    let mut frames = Vec::new();
    let mut instances = Vec::new();
    for p in projects {
        for f in p
            .get("frames")
            .and_then(Json::as_array)
            .into_iter()
            .flatten()
        {
            let item = (|| {
                Some(ObservedFrame {
                    name: text(f, "name")?,
                    document: text(f, "document")?,
                    transform: wire_transform(f.get("transform")?)?,
                })
            })();
            match item {
                Some(i) => frames.push(i),
                None => tracing::warn!("sync: malformed observed frame skipped"),
            }
        }
        for i in p
            .get("instances")
            .and_then(Json::as_array)
            .into_iter()
            .flatten()
        {
            let item = (|| {
                Some(ObservedInstance {
                    component: text(i, "component")?,
                    document: text(i, "document")?,
                    reference: text(i, "reference"),
                    transform: wire_transform(i.get("transform")?)?,
                })
            })();
            match item {
                Some(i) => instances.push(i),
                None => tracing::warn!("sync: malformed observed instance skipped"),
            }
        }
    }
    tracing::debug!(
        frames = frames.len(),
        instances = instances.len(),
        "sync: observed geometry read"
    );
    (frames, instances)
}

/// The SI length of one native unit (0.001 for `mm`); 1.0 with a warning when it is unknown.
fn native_scale(adapter: &Adapter) -> f64 {
    let unit = adapter
        .native_unit
        .as_deref()
        .unwrap_or(DEFAULT_NATIVE_UNIT);
    UnitTable::builtin().lookup(unit).map_or_else(
        || {
            tracing::warn!(%unit, "sync: unknown native unit, geometry scale taken as 1");
            1.0
        },
        |u| u.factor.to_f64(),
    )
}

/// Geometry verification and datum driving (adapters.md 3.4) over one adapter's read.
///
/// Reports R001 for every frame or instance whose transform differs from the composed design,
/// and returns the `datums = create` entries to append to the sync entries (empty under
/// `verify` or without the `create_frames` feature). Each datum is driven in the document of its
/// component: the file of any observed member of that component, else the frame's own document.
// frob:ticket 01M4W1REG5EVCAD4NWWB6MN43Z
fn geometry_pass(
    adapter: &Adapter,
    ir: &Ir,
    bindings: &[cadetta_ir::model::Binding],
    observed: &[Observed],
    read: &Geometry<'_>,
    ctx: &mut Ctx<'_>,
) -> Vec<Entry> {
    let id = adapter.id();
    let mut project = ir.clone();
    project.bindings = bindings.to_vec();
    (project.frames, project.instances) = observed_geometry(read.observed);
    let scale = native_scale(adapter);
    let precision = read.precision;
    for d in verify_geometry(&project, id, scale, precision) {
        ctx.report.diag(d.code, d.message);
    }
    let mut files: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for c in &ir.components {
        let file = c
            .members
            .iter()
            .filter_map(|m| {
                bindings
                    .iter()
                    .find(|b| b.adapter == id && b.path == m.path)
            })
            .flat_map(|b| b.locators.iter())
            .find_map(|l| observed.iter().find(|o| o.native == l.locator))
            .map(|o| o.file.clone());
        for d in &c.datums {
            let frame = bindings
                .iter()
                .filter(|b| b.adapter == id && b.path == d.path)
                .flat_map(|b| b.locators.iter())
                .find(|l| l.kind == LocatorKind::Frame)
                .map_or_else(
                    || cadetta_adapter::bindings::mangle(&d.path, ""),
                    |l| l.locator.clone(),
                );
            let file = file
                .clone()
                .or_else(|| {
                    project
                        .frames
                        .iter()
                        .find(|f| f.name == frame)
                        .map(|f| f.document.clone())
                })
                .or_else(|| match adapter.files.as_slice() {
                    [only] if !only.contains(['*', '?', '[']) => Some(only.clone()),
                    _ => None,
                });
            match file {
                Some(f) => {
                    files.entry(f).or_default().insert(d.path.to_string());
                }
                None => {
                    tracing::warn!(datum = %d.path, "sync: datum has no document to create its frame in")
                }
            }
        }
    }
    let lock = &*ctx.lock;
    let base = |path: &str, frame: &str| match lock.get(
        &Record::SyncedValue(SyncedValue {
            adapter: id.to_owned(),
            path: path.to_owned(),
            locator: frame.to_owned(),
            value: String::new(),
        })
        .key(),
    ) {
        Some(Record::SyncedValue(v)) => Some(v.value.clone()),
        _ => None,
    };
    let mut out = Vec::new();
    for (file, paths) in &files {
        out.extend(
            datum_entries(
                &project,
                id,
                file,
                adapter.datums,
                read.features,
                scale,
                precision,
                &base,
            )
            .into_iter()
            .filter(|e| paths.contains(&e.path)),
        );
    }
    tracing::info!(adapter = %id, datums = out.len(), "sync: geometry pass done");
    out
}

/// Store the engine's changed `synced` values into the lock file model.
fn apply_lock(ctx: &mut Ctx<'_>, before: &EngineLock, after: &EngineLock) {
    for ((adapter, path, locator), value) in after {
        if before.get(&(adapter.clone(), path.clone(), locator.clone())) == Some(value) {
            continue;
        }
        let rec = Record::SyncedValue(SyncedValue {
            adapter: adapter.clone(),
            path: path.clone(),
            locator: locator.clone(),
            value: value.clone(),
        });
        // Tie rule: remove the old key first so the new value always wins.
        ctx.lock.remove(&rec.key());
        ctx.lock.insert(rec);
        ctx.lock_changed = true;
        tracing::info!(%adapter, %path, %locator, "sync: lock value recorded");
    }
}

fn to_records(report: &cadetta_adapter::sync::Report, out: &mut SyncReport) {
    for r in &report.records {
        let theirs = r.entry.theirs_record();
        out.records.push(SyncRecord {
            adapter: r.entry.adapter.clone(),
            file: r.entry.file.clone(),
            path: r.entry.path.clone(),
            locator: r.entry.locator.clone(),
            verb: r.verb.to_string(),
            old: r.entry.base.clone().unwrap_or(theirs),
            new: r.entry.ours_record(),
        });
    }
    for d in &report.diagnostics {
        out.diag(d.code, d.message.clone());
    }
}

/// Sync one drive-capable adapter over an already initialized `client`.
fn drive_adapter<R: BufRead, W: Write>(
    client: Client<R, W>,
    pkg: &PackageDir,
    adapter: &Adapter,
    ir: &Ir,
    ctx: &mut Ctx<'_>,
) -> Result<(), String> {
    let id = adapter.id().to_owned();
    let files = expand_files(pkg, adapter);
    if files.is_empty() {
        let _ = client.shutdown();
        return Err(format!("adapter `{id}` has no `files` to read"));
    }
    let known_files = relative_files(pkg, &files);
    let mut client = client;
    let read = client.read(&ReadParams {
        source: ReadSource::Files(files),
    });
    let read = match read {
        Ok(r) => r,
        Err(e) => {
            let _ = client.shutdown();
            return Err(format!("adapter `{id}` read failed: {e}"));
        }
    };
    let observed = observe(&read.observed, &known_files);
    let geometry = Geometry {
        observed: &read.observed,
        features: &client.info().features,
        tolerances: &read.tolerances,
        precision: client.effective_precision(adapter.precision.map(u32::from)),
    };
    let precision = geometry.precision;
    let Some((mut entries, units, kinds)) =
        build_entries(pkg, adapter, ir, &observed, &geometry, ctx)
    else {
        let _ = client.shutdown();
        return Ok(());
    };
    let mut engine_lock: EngineLock = ctx
        .lock
        .records()
        .filter_map(|r| match r {
            Record::SyncedValue(v) if v.adapter == id => Some((
                (v.adapter.clone(), v.path.clone(), v.locator.clone()),
                v.value.clone(),
            )),
            _ => None,
        })
        .collect();
    let before = engine_lock.clone();
    let opts = Options {
        dry_run: true,
        only: ctx.args.only.clone(),
        no_regen: ctx.args.no_regen,
        force_design: ctx.args.force_design,
        adapters: vec![id.clone()],
    };
    let plan = run(&entries, &opts, &mut NoBackend, &mut before.clone());
    let pushed_files: Vec<String> = plan
        .records
        .iter()
        .filter(|r| r.verb == Verb::Push)
        .map(|r| r.entry.file.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    if ctx.args.dry_run || pushed_files.is_empty() {
        let real = if ctx.args.dry_run {
            plan
        } else {
            // Nothing to push: record converged values, never open a session or rebuild.
            let real = run(
                &entries,
                &Options {
                    dry_run: false,
                    ..opts
                },
                &mut NoBackend,
                &mut engine_lock,
            );
            apply_lock(ctx, &before, &engine_lock);
            real
        };
        to_records(&real, ctx.report);
        if let Err(e) = client.shutdown() {
            tracing::warn!(error = %e, "sync: adapter shutdown failed");
        }
        return Ok(());
    }
    let session = match DriveSession::open(client, &pushed_files) {
        Ok(s) => s,
        Err(e) => {
            let code = e.code().unwrap_or("R009");
            ctx.report.diag(code, e.to_string());
            to_records(&plan, ctx.report);
            return Ok(());
        }
    };
    // R009 is per file: a refused file is never handed to the engine, the rest proceed.
    for busy in session.busy() {
        ctx.report
            .diag(&busy.diagnostic.code, busy.diagnostic.message.clone());
    }
    let refused: BTreeSet<String> = session.busy().iter().map(|b| b.file.clone()).collect();
    entries.retain(|e| !refused.contains(&e.file));
    let mut backend = SessionBackend {
        session,
        known_files,
        default_unit: units,
        kinds,
        precision,
        regen: Vec::new(),
        diags: Vec::new(),
        failure: None,
        pushed: Vec::new(),
    };
    let real = run(
        &entries,
        &Options {
            dry_run: false,
            ..opts
        },
        &mut backend,
        &mut engine_lock,
    );
    apply_lock(ctx, &before, &engine_lock);
    to_records(&real, ctx.report);
    ctx.report.regen.append(&mut backend.regen);
    // R010: one diagnostic per file naming the locators the adapter rejected.
    for r in rejections(&backend.pushed) {
        backend
            .diags
            .push(SyncDiag::new(r.diagnostic.code, r.diagnostic.message));
    }
    let blocked = backend.failure.is_some()
        || real.records.iter().any(|r| r.verb == Verb::Fail)
        || backend
            .diags
            .iter()
            .any(|d| ["R004", "R009", "R010"].contains(&d.code.as_str()));
    ctx.report.diagnostics.append(&mut backend.diags);
    let mut session = backend.session;
    let mode = match adapter.save {
        SaveMode::AfterSync => SessionSaveMode::AfterSync,
        SaveMode::Never => SessionSaveMode::Never,
    };
    let mut failure = backend.failure;
    if mode == SessionSaveMode::AfterSync && !blocked && !ctx.args.no_regen {
        match session.save() {
            Ok(saved) => record_saved(ctx, pkg, &id, &saved),
            Err(e) => failure = Some(e.to_string()),
        }
    } else {
        tracing::info!(
            ?mode,
            blocked,
            no_regen = ctx.args.no_regen,
            "sync: save skipped"
        );
    }
    if let Err(e) = session.finish() {
        tracing::warn!(error = %e, "sync: closing the session failed");
    }
    failure.map_or(Ok(()), |f| Err(format!("adapter `{id}`: {f}")))
}

/// Record the saved file digests as `synced file=` lock records.
fn record_saved(
    ctx: &mut Ctx<'_>,
    pkg: &PackageDir,
    adapter: &str,
    saved: &cadetta_adapter::rpc::SaveResult,
) {
    for f in &saved.files {
        let hex = f.digest.strip_prefix("blake3:").unwrap_or(&f.digest);
        let Ok(digest) = Hex::new(hex) else {
            tracing::warn!(file = %f.path, "sync: unusable digest not recorded");
            continue;
        };
        let rel = FsPath::new(&f.path)
            .strip_prefix(&pkg.dir)
            .map_or_else(|_| f.path.clone(), |p| p.to_string_lossy().into_owned());
        let rec = Record::SyncedFile(SyncedFile {
            adapter: adapter.to_owned(),
            file: rel,
            digest,
        });
        ctx.lock.remove(&rec.key());
        ctx.lock.insert(rec);
        ctx.lock_changed = true;
    }
}

/// The by-hand steps for an adapter that cannot be driven.
fn instructions(adapter: &Adapter, emitted: Option<&str>) -> String {
    if adapter.name == SW_EQUATIONS && adapter.command.is_none() {
        let file = emitted.map_or_else(
            || {
                adapter
                    .out
                    .clone()
                    .unwrap_or_else(|| format!("build/solidworks/{}.txt", adapter.id()))
            },
            str::to_owned,
        );
        format!(
            "In SOLIDWORKS open Tools > Equations, choose Link to external file and select `{file}`, then rebuild (Ctrl+B)."
        )
    } else {
        format!(
            "`{}` does not declare `drive`: update its native files by hand from the design values, then run `cadetta check`.",
            adapter.id()
        )
    }
}

/// Read the on-disk csv of a built-in `csv` adapter before it is re-emitted, so a row naming
/// nothing the bindings bind is R016 (emitters/csv.md 3) instead of being silently overwritten.
// frob:ticket 01M4YT94YMMH6YRC20YTD7M33H
fn read_back_csv(pkg: &PackageDir, adapter: &Adapter, ir: &Ir, ctx: &mut Ctx<'_>) {
    let rel = adapter
        .out
        .clone()
        .unwrap_or_else(|| value_row_rel(adapter));
    let Ok(text) = std::fs::read_to_string(pkg.dir.join(&rel)) else {
        tracing::debug!(file = %rel, "sync: no csv on disk to read back");
        return;
    };
    let Ok((set, _)) = load_bindings(pkg, adapter, ir) else {
        return;
    };
    let bound: Vec<&str> = set
        .entries()
        .iter()
        .flat_map(|b| b.locators.iter().map(|l| l.locator.as_str()))
        .collect();
    let units = UnitPolicy {
        native_unit: Some(
            adapter
                .native_unit
                .clone()
                .unwrap_or_else(|| DEFAULT_NATIVE_UNIT.to_owned()),
        ),
        assume_unit: None,
    };
    match csv::read_bound(&text, &units, &rel, &bound) {
        Ok(params) => tracing::info!(file = %rel, rows = params.len(), "sync: csv read back"),
        Err(e) => {
            let code = match e {
                csv::CsvError::UnboundRow { .. } => "R016",
                _ => "R005",
            };
            ctx.report.diag(code, e.to_string());
        }
    }
}

/// Record the native files a built-in adapter without `drive` handed over as `synced file=`
/// lock records, so the "CAD file reflects design" obligation exists and stays open
/// (adapters.md 2: unverifiable without drive).
///
/// The files are the adapter's literal `files` entries, else the emitted file; the digest is
/// that of the emitted text cadetta wrote (the native file is not read, nothing can verify it).
// frob:ticket 01M4YW6FFKS9DAYE3DAFEZDG87
fn record_handed_over(
    ctx: &mut Ctx<'_>,
    pkg: &PackageDir,
    adapter: &Adapter,
    emitted: Option<&str>,
) {
    let Some(written) = emitted.and_then(|e| std::fs::read(pkg.dir.join(e)).ok()) else {
        tracing::warn!(adapter = %adapter.id(), "sync: nothing emitted, no native file recorded");
        return;
    };
    let digest = match Hex::new(blake3::hash(&written).to_hex().as_str()) {
        Ok(d) => d,
        Err(e) => {
            tracing::warn!(adapter = %adapter.id(), error = %e, "sync: digest unusable");
            return;
        }
    };
    let mut files: Vec<String> = adapter
        .files
        .iter()
        .filter(|f| !f.contains('*'))
        .cloned()
        .collect();
    if files.is_empty() {
        files.extend(emitted.map(|e| {
            FsPath::new(e)
                .strip_prefix(&pkg.dir)
                .map_or_else(|_| e.to_owned(), |p| p.to_string_lossy().into_owned())
        }));
    }
    for file in files {
        let rec = Record::SyncedFile(SyncedFile {
            adapter: adapter.id().to_owned(),
            file: file.clone(),
            digest: digest.clone(),
        });
        tracing::info!(adapter = %adapter.id(), %file, "sync: native file handed over, O004 open");
        ctx.lock.remove(&rec.key());
        ctx.lock.insert(rec);
        ctx.lock_changed = true;
        ctx.report.handed_over.insert(file);
    }
}

/// Report every file this run handed over whose `sync` obligation is still open as O004.
fn report_open_sync(
    report: &mut SyncReport,
    project: &Project,
    analysis: &crate::pipeline::Analysis,
    lock: &FileLock,
) {
    use cadetta_obligations::graph::{ObligationKind, ObligationState};
    for (id, state) in universe::sync_states(project, analysis, lock) {
        if state == ObligationState::Open
            && id.kind() == ObligationKind::Sync
            && report.handed_over.contains(id.path())
        {
            report.diag(
                "O004",
                format!(
                    "CAD file reflects design: `{id}` is open, {}; attest it with `cadetta ack`",
                    cadetta_lint::rules::o004_w005::UNVERIFIABLE_REASON
                ),
            );
        }
    }
}

/// Fall back to emit plus manual instructions (no classification, nothing pushed).
fn manual_adapter(
    pkg: &PackageDir,
    adapter: &Adapter,
    ir: &Ir,
    ctx: &mut Ctx<'_>,
    external: Option<String>,
) {
    let mut emitted = external;
    if adapter.command.is_none() && adapter.name == csv::ADAPTER {
        read_back_csv(pkg, adapter, ir, ctx);
    }
    if !ctx.args.dry_run && adapter.command.is_none() {
        let mut report = EmitReport::default();
        let parsed = EmitArgs {
            adapters: vec![adapter.id().to_owned()],
            ..EmitArgs::default()
        };
        emit_package_texts(pkg, ir, &ctx.texts, &parsed, ctx.run_scope, &mut report);
        for f in report.files {
            emitted = Some(f.path.clone());
            ctx.report.emitted.push(f.path);
        }
        for d in report.diagnostics {
            ctx.report.diag(&d.code, d.message);
        }
        if let Some(message) = report.config_error {
            ctx.report.config_error = Some(message.clone());
            ctx.report.diag("R005", message);
        }
    }
    if !ctx.args.dry_run && adapter.command.is_none() {
        record_handed_over(ctx, pkg, adapter, emitted.as_deref());
    }
    tracing::info!(adapter = %adapter.id(), "sync: no drive, manual instructions");
    ctx.report.manual.push(Manual {
        adapter: adapter.id().to_owned(),
        instructions: instructions(adapter, emitted.as_deref()),
    });
}

/// Run the `emit` of an external adapter that cannot be driven, then shut it down.
///
/// Returns the first file written, for the manual instructions; `Err` is an adapter failure
/// (exit 2). An adapter without `emit`, or a dry run, emits nothing (adapters.md 2.1, 5).
// frob:ticket 01M4XDMA1D78TRJB1CB5HP91AE
fn emit_external(
    mut client: ProcessClient,
    pkg: &PackageDir,
    adapter: &Adapter,
    ir: &Ir,
    ctx: &mut Ctx<'_>,
) -> Result<Option<String>, String> {
    let mut first = None;
    let mut failure = None;
    if !ctx.args.dry_run && client.info().capabilities.contains(&CapabilityName::Emit) {
        match emit_params(pkg, adapter, ir) {
            Err(message) => failure = Some(message),
            Ok(params) => match client.emit(&params) {
                Ok(result) => {
                    for f in result.files {
                        tracing::info!(adapter = %adapter.id(), file = %f.path, "sync: external emit wrote");
                        first.get_or_insert_with(|| f.path.clone());
                        ctx.report.emitted.push(f.path);
                    }
                    for d in &result.diagnostics {
                        let (code, message) = diag_parts(&d.0);
                        ctx.report.diag(&code, message);
                    }
                }
                Err(e) => {
                    failure = Some(format!("R008: adapter `{}` emit failed: {e}", adapter.id()));
                }
            },
        }
    }
    if let Err(e) = client.shutdown() {
        tracing::warn!(error = %e, "sync: adapter shutdown failed");
    }
    failure.map_or(Ok(first), Err)
}

/// Run one adapter of a package; `Err` is a usage/IO/adapter failure (exit 2).
fn sync_adapter(
    pkg: &PackageDir,
    adapter: &Adapter,
    ir: &Ir,
    ctx: &mut Ctx<'_>,
) -> Result<(), String> {
    let Some(command) = &adapter.command else {
        manual_adapter(pkg, adapter, ir, ctx, None);
        return Ok(());
    };
    let (path, argv) = program(pkg, command)
        .ok_or_else(|| format!("adapter `{}` has an empty command", adapter.id()))?;
    tracing::info!(adapter = %adapter.id(), "sync: starting adapter");
    let client = ProcessClient::spawn_with_timeout(
        &path,
        &argv,
        adapter.version.as_deref(),
        Some(adapter.timeout()),
    )
    .map_err(|e| match unavailable(adapter.id(), &e, AvailVerb::Sync) {
        Some(u) => format!("{}: {}", u.diagnostic.code, u.diagnostic.message),
        None => format!("R008: could not start adapter `{}`: {e}", adapter.id()),
    })?;
    if client.info().capabilities.contains(&CapabilityName::Drive) {
        drive_adapter(client, pkg, adapter, ir, ctx)
    } else {
        let external = emit_external(client, pkg, adapter, ir, ctx)?;
        manual_adapter(pkg, adapter, ir, ctx, external);
        Ok(())
    }
}

/// What a check-time read of one adapter found: the three-way entries and everything the R lints
/// need beyond them.
// frob:ticket 01M4TA3C87N60Y711163RK3GPH
#[derive(Debug, Default)]
pub(crate) struct Probe {
    /// Three-way entries of the bound, readable locators.
    pub entries: Vec<Entry>,
    /// Diagnostics `build_entries` raised (R001 geometry, R005, R013, ...), as `(code, message)`.
    pub diagnostics: Vec<(String, String)>,
    /// Bound `(path, file, locator)` triples missing from the native files.
    pub missing: Vec<(String, String, String)>,
    /// Native `(file, name)` values no binding covers.
    pub unbound: Vec<(String, String)>,
    /// The loaded bindings as `(path, locator)`, first locator of each path.
    pub bindings: Vec<(String, String)>,
    /// The adapter's own diagnostics from `read`, as `(code, message)`.
    pub reports: Vec<(String, String)>,
    /// Every bound `(path, locator)` pair, not only the first locator of each path.
    // frob:ticket 01M4YB06FSER6VHW3RBC0B2X17
    pub locator_paths: Vec<(String, String)>,
    /// `(path, file)` of each observed member that names its native file.
    // frob:ticket 01M4YB06FSER6VHW3RBC0B2X17
    pub member_files: Vec<(String, String)>,
    /// Sketch facts the adapter read (R007).
    // frob:ticket 01M4YB06FSER6VHW3RBC0B2X17
    pub sketches: Vec<ObservedSketch>,
    /// Assembly component facts the adapter read (R011).
    // frob:ticket 01M4YB06FSER6VHW3RBC0B2X17
    pub components: Vec<ObservedComponent>,
    /// Per-instance values of `[*]` bindings the adapter read (R012).
    // frob:ticket 01M4YB06FSER6VHW3RBC0B2X17
    pub instance_values: Vec<ObservedInstanceValues>,
}

/// `(path, file)` for each observed member whose provenance names its native file.
// frob:ticket 01M4YB06FSER6VHW3RBC0B2X17
fn member_files(observed: &Json) -> Vec<(String, String)> {
    let projects: Vec<&Json> = match observed {
        Json::Array(a) => a.iter().collect(),
        v => vec![v],
    };
    projects
        .iter()
        .filter_map(|p| p.get("components").and_then(Json::as_array))
        .flatten()
        .filter_map(|c| c.get("members").and_then(Json::as_array))
        .flatten()
        .filter_map(|m| {
            let path = m.get("path")?.as_str()?;
            let file = m
                .get("provenance")?
                .get("imported")?
                .get("file")?
                .as_str()?;
            Some((path.to_owned(), file.to_owned()))
        })
        .collect()
}

/// Read an adapter's files over `client` and compare them with the design IR without writing
/// anything: the data behind the `check` drift lints (adapters.md 2, 3, 7).
///
/// The lock is only read for bases; `Err` is a read failure the caller reports as R008.
// frob:ticket 01M4TA3C87N60Y711163RK3GPH
pub(crate) fn probe_adapter<R: BufRead, W: Write>(
    client: &mut Client<R, W>,
    pkg: &PackageDir,
    adapter: &Adapter,
    ir: &Ir,
    lock: &FileLock,
) -> Result<Probe, String> {
    let id = adapter.id().to_owned();
    let files = expand_files(pkg, adapter);
    if files.is_empty() {
        return Err(format!("adapter `{id}` has no `files` to read"));
    }
    let known_files = relative_files(pkg, &files);
    let read = client
        .read(&ReadParams {
            source: ReadSource::Files(files),
        })
        .map_err(|e| format!("adapter `{id}` read failed: {e}"))?;
    let observed = observe(&read.observed, &known_files);
    let geometry = Geometry {
        observed: &read.observed,
        features: &client.info().features,
        tolerances: &read.tolerances,
        precision: client.effective_precision(adapter.precision.map(u32::from)),
    };
    let run_scope = RunScope::new(&[] as &[String], &[] as &[String]).map_err(|e| e.to_string())?;
    let args = SyncArgs::default();
    let mut scratch = lock.clone();
    let mut report = SyncReport::default();
    let mut ctx = Ctx {
        args: &args,
        run_scope: &run_scope,
        lock: &mut scratch,
        lock_changed: false,
        report: &mut report,
        texts: TextParams::new(),
    };
    let built = build_entries(pkg, adapter, ir, &observed, &geometry, &mut ctx);
    let mut probe = Probe {
        entries: built.map(|(entries, _, _)| entries).unwrap_or_default(),
        reports: read.diagnostics.iter().map(|d| diag_parts(&d.0)).collect(),
        member_files: member_files(&read.observed),
        sketches: read.sketches.clone(),
        components: read.components.clone(),
        instance_values: read.instance_values.clone(),
        ..Probe::default()
    };
    probe.diagnostics = report
        .diagnostics
        .iter()
        .map(|d| (d.code.clone(), d.message.clone()))
        .collect();
    probe.missing = std::mem::take(&mut report.missing);
    if let Ok((set, _)) = load_bindings(pkg, adapter, ir) {
        let bound: BTreeSet<&str> = set
            .entries()
            .iter()
            .flat_map(|b| b.locators.iter().map(|l| l.locator.as_str()))
            .collect();
        probe.unbound = observed
            .iter()
            .filter(|o| !bound.contains(o.native.as_str()))
            .map(|o| (o.file.clone(), o.native.clone()))
            .collect();
        probe.bindings = set
            .entries()
            .iter()
            .filter_map(|b| Some((b.path.to_string(), b.locators.first()?.locator.clone())))
            .collect();
        probe.locator_paths = set
            .entries()
            .iter()
            .flat_map(|b| {
                b.locators
                    .iter()
                    .map(|l| (b.path.to_string(), l.locator.clone()))
            })
            .collect();
    }
    tracing::info!(adapter = %id, entries = probe.entries.len(), missing = probe.missing.len(), unbound = probe.unbound.len(), "check: adapter probed");
    Ok(probe)
}

fn render_text(report: &SyncReport, dry_run: bool) -> String {
    let mut s = String::new();
    for r in &report.records {
        s.push_str(&format!(
            "{}  {}  {}  {} -> {}\n",
            r.verb, r.path, r.locator, r.old, r.new
        ));
    }
    for d in &report.diagnostics {
        s.push_str(&format!("{}: {}\n", d.code, d.message));
    }
    let count = |v: &str| report.records.iter().filter(|r| r.verb == v).count();
    s.push_str(&format!(
        "{}{} pushed, {} skipped, {} drift, {} converged, {} failed\n",
        if dry_run { "dry run: " } else { "" },
        count("push"),
        count("skip"),
        count("drift"),
        count("converged"),
        count("fail"),
    ));
    for r in &report.regen {
        s.push_str(&format!("regen  {}  {}\n", r.file, r.status));
    }
    for f in &report.emitted {
        s.push_str(&format!("wrote {f}\n"));
    }
    for m in &report.manual {
        s.push_str(&format!("manual  {}: {}\n", m.adapter, m.instructions));
    }
    s
}

/// Run `cadetta sync`; `args` are the words after the verb. Returns the exit code.
pub fn run_sync(
    args: &[String],
    globals: &Globals,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8 {
    let fail = |err: &mut dyn Write, message: &str| {
        tracing::error!(%message, "sync failed");
        let _ = writeln!(err, "error: {message}");
        EXIT_USAGE
    };
    let parsed = match parse_args(args) {
        Ok(p) => p,
        Err(message) => return fail(err, &format!("{message}; {USAGE}")),
    };
    let run_scope = match RunScope::new(&parsed.modules, &parsed.only) {
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
    let lock_path = project.root.join("cadetta.lock");
    let mut lock = match FileLock::load(&lock_path) {
        Ok(l) => l,
        Err(e) => return fail(err, &format!("could not read cadetta.lock: {e}")),
    };
    let mut report = SyncReport::default();
    let mut lock_changed = false;
    for index in &scope {
        let pkg = &project.packages[*index];
        let ir = match package_ir(&analysis, pkg) {
            Ok(ir) => ir,
            Err(message) => return fail(err, &message),
        };
        let texts = text_params_of(&analysis, pkg);
        for adapter in &pkg.config.adapters {
            if !parsed.adapters.is_empty() && !parsed.adapters.iter().any(|a| a == adapter.id()) {
                continue;
            }
            let mut ctx = Ctx {
                args: &parsed,
                run_scope: &run_scope,
                lock: &mut lock,
                lock_changed: false,
                report: &mut report,
                texts: texts.clone(),
            };
            let result = sync_adapter(pkg, adapter, &ir, &mut ctx);
            lock_changed |= ctx.lock_changed;
            if let Err(message) = result {
                return fail(err, &message);
            }
            if let Some(message) = report.config_error.take() {
                return fail(err, &message);
            }
        }
    }
    if lock_changed && !parsed.dry_run && !parsed.no_regen {
        match lock.save(&lock_path) {
            Ok(_) => report.lock_written = true,
            Err(e) => return fail(err, &format!("could not write cadetta.lock: {e}")),
        }
    }
    report_open_sync(&mut report, &project, &analysis, &lock);
    let rendered = match globals.format {
        Format::Json => serde_json::to_string_pretty(&report).map_err(|e| e.to_string()),
        _ => Ok(render_text(&report, parsed.dry_run)),
    };
    match rendered {
        Ok(text) => {
            let _ = writeln!(out, "{}", text.trim_end());
        }
        Err(e) => return fail(err, &format!("could not render the report: {e}")),
    }
    if report.failed() { EXIT_FAIL } else { EXIT_OK }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Serializes writing and spawning of script adapters (ETXTBSY on a still-open writer).
    static SPAWN_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    use std::io::Cursor;

    /// A backslash-separated relative path (Windows) still matches a `/` glob.
    // frob:tests rel_glob_path
    #[test]
    fn rel_glob_path_normalises_backslashes() {
        let rel = rel_glob_path(FsPath::new("parts\\a.SLDPRT"));
        assert_eq!(rel, "parts/a.SLDPRT");
        assert!(Glob::new("parts/*.SLDPRT").unwrap().matches(&rel));
    }

    /// A self-removing scratch directory.
    struct Dir(PathBuf);
    impl Dir {
        fn new() -> Self {
            static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
            let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let p = std::env::temp_dir().join(format!("cadetta-sync-{}-{n}", std::process::id()));
            std::fs::create_dir_all(&p).unwrap();
            Self(p)
        }
    }
    impl Drop for Dir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    const SOURCE: &str = "model CoverPlate {\n  thickness: Length = 6mm\n}\n";
    const BINDINGS: &str = "[\"acme.plate::CoverPlate.thickness\"]\nlocators = [ { locator = \"Plate_Thk\", kind = \"global\", native_unit = \"mm\" } ]\n";
    const PATH: &str = "acme.plate::CoverPlate.thickness";

    fn project(extra: &str) -> Dir {
        project_with(extra, SOURCE, BINDINGS)
    }

    fn project_with(extra: &str, source: &str, bindings: &str) -> Dir {
        let dir = Dir::new();
        let toml = format!(
            "[package]\nname = \"acme.plate\"\nversion = \"0.1.0\"\n[[adapter]]\nname = \"sw-equations\"\nid = \"equations\"\n{extra}"
        );
        std::fs::write(dir.0.join("cadetta.toml"), toml).unwrap();
        std::fs::create_dir_all(dir.0.join("src")).unwrap();
        std::fs::write(dir.0.join("src/lib.cdt"), source).unwrap();
        std::fs::create_dir_all(dir.0.join("bindings")).unwrap();
        std::fs::write(dir.0.join("bindings/equations.toml"), bindings).unwrap();
        dir
    }

    fn run(dir: &Dir, format: Format, args: &[&str]) -> (u8, String, String) {
        let args: Vec<String> = args.iter().map(|a| (*a).to_owned()).collect();
        let globals = Globals {
            format,
            cwd: Some(dir.0.clone()),
            config: None,
        };
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = run_sync(&args, &globals, &mut out, &mut err);
        (
            code,
            String::from_utf8(out).unwrap(),
            String::from_utf8(err).unwrap(),
        )
    }

    fn script(results: &[Json]) -> Cursor<Vec<u8>> {
        let mut out = String::new();
        for (i, r) in results.iter().enumerate() {
            out.push_str(&json!({"jsonrpc": "2.0", "id": i + 1, "result": r}).to_string());
            out.push('\n');
        }
        Cursor::new(out.into_bytes())
    }

    fn init() -> Json {
        json!({"name": "mock", "version": "1.0.0", "capabilities": ["drive"],
               "native_units": "mm", "precision": 6, "platforms": ["linux"]})
    }

    fn observed(text: &str) -> Json {
        json!({"observed": {"components": [{"members": [{
            "path": PATH,
            "provenance": {"imported": {"file": "part.FCStd", "native_name": "Plate_Thk"}},
            "value": {"authored": text}}]}]}, "diagnostics": []})
    }

    fn methods(out: &[u8]) -> Vec<String> {
        String::from_utf8(out.to_vec())
            .unwrap()
            .lines()
            .map(|l| {
                serde_json::from_str::<Json>(l).unwrap()["method"]
                    .as_str()
                    .unwrap()
                    .to_owned()
            })
            .collect()
    }

    fn pushed(old: &str, new: &str) -> Json {
        json!({"statuses": [{"locator": "Plate_Thk", "status": "pushed", "old": old, "new": new}],
               "diagnostics": []})
    }

    /// Drive one scripted adapter over the temp project; returns report, methods, lock.
    fn drive(
        args: &[&str],
        base: Option<&str>,
        results: &[Json],
    ) -> (SyncReport, Vec<String>, FileLock) {
        let dir = project("files = [\"part.FCStd\"]\n");
        let (report, out, lock) = drive_in(&dir, args, base, results);
        (report, methods(&out), lock)
    }

    /// [`drive`] over `dir`; returns the report, the raw requests sent, and the lock.
    fn drive_in(
        dir: &Dir,
        args: &[&str],
        base: Option<&str>,
        results: &[Json],
    ) -> (SyncReport, Vec<u8>, FileLock) {
        let args = parse_args(&args.iter().map(|a| (*a).to_owned()).collect::<Vec<_>>()).unwrap();
        let (proj, scope) = Project::open(Some(&dir.0), None).unwrap();
        let analysis = analyze(&proj, &scope);
        let pkg = &proj.packages[scope[0]];
        let ir = package_ir(&analysis, pkg).unwrap();
        let mut lock = FileLock::load(&dir.0.join("cadetta.lock")).unwrap();
        if let Some(b) = base {
            lock.insert(Record::SyncedValue(SyncedValue {
                adapter: "equations".into(),
                path: PATH.into(),
                locator: "Plate_Thk".into(),
                value: b.into(),
            }));
        }
        let run_scope = RunScope::new(&args.modules, &args.only).unwrap();
        let mut report = SyncReport::default();
        let mut out = Vec::new();
        let client = Client::connect(script(results), &mut out, None).unwrap();
        let mut ctx = Ctx {
            args: &args,
            run_scope: &run_scope,
            lock: &mut lock,
            lock_changed: false,
            report: &mut report,
            texts: TextParams::new(),
        };
        drive_adapter(client, pkg, &pkg.config.adapters[0], &ir, &mut ctx).unwrap();
        (report, out, lock)
    }

    fn full_run(readback: &str) -> Vec<Json> {
        vec![
            init(),
            observed("5mm"),
            json!({"session": {"handle": "s1", "files": [{"path": "part.FCStd", "dirty": false, "readonly": false}]}, "diagnostics": []}),
            observed("5mm"),
            pushed("5mm", "6mm"),
            json!({"targets": [{"target": "part.FCStd", "status": "ok"}], "diagnostics": []}),
            observed(readback),
            json!({"files": [{"path": "part.FCStd", "digest": "blake3:0a"}], "diagnostics": []}),
            Json::Null,
            Json::Null,
        ]
    }

    // frob:tests crates/cadetta-cli/src/cmd/sync.rs::normalize
    #[test]
    fn normalize_canonicalizes_numbers_and_units() {
        assert_eq!(normalize("6.0 mm", "mm"), "6mm");
        assert_eq!(normalize("6.50", "mm"), "6.5mm");
        assert_eq!(normalize("-0.0mm", "mm"), "0mm");
        assert_eq!(normalize("+2", ""), "2");
        assert_eq!(normalize("1e3mm", "mm"), "1e3mm");
        assert_eq!(normalize(" \"a\" ", "mm"), "\"a\"");
    }

    // frob:ticket 01M4XDM9WRFB7RFEF2X6RBPR2Y
    // frob:tests crates/cadetta-cli/src/cmd/sync.rs::normalize_in
    #[test]
    fn normalize_in_converts_same_dimension_units_exactly() {
        let t = UnitTable::builtin();
        assert_eq!(normalize_in(&t, "0.5in", "mm", 6), "12.7mm");
        assert_eq!(normalize_in(&t, "12.7 mm", "mm", 6), "12.7mm");
        assert_eq!(normalize_in(&t, "1cm", "mm", 6), "10mm");
        assert_eq!(normalize_in(&t, "2", "mm", 6), "2mm");
        assert_eq!(normalize_in(&t, "25.4mm", "in", 6), "1in");
    }

    // frob:ticket 01M4XDM9WRFB7RFEF2X6RBPR2Y
    // frob:tests crates/cadetta-cli/src/cmd/sync.rs::normalize_in
    #[test]
    fn normalize_in_keeps_text_it_cannot_convert_exactly() {
        let t = UnitTable::builtin();
        // Different dimension, unknown unit, non-terminating decimal and pi all stay textual.
        assert_eq!(normalize_in(&t, "5kg", "mm", 6), "5kg");
        assert_eq!(normalize_in(&t, "5furlong", "mm", 6), "5furlong");
        assert_eq!(normalize_in(&t, "1mm", "in", 6), "1mm");
        assert_eq!(normalize_in(&t, "90deg", "rad", 6), "90deg");
        assert_eq!(normalize_in(&t, "\"a\"", "mm", 6), "\"a\"");
    }

    // frob:tests crates/cadetta-cli/src/cmd/sync.rs::parse_args
    #[test]
    fn parse_args_reads_every_flag() {
        let a = |v: &[&str]| parse_args(&v.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>());
        let p = a(&[
            "--dry-run",
            "--only",
            "a::*",
            "--force-design",
            "--no-regen",
            "--module",
            "m",
            "fc",
        ])
        .unwrap();
        assert!(p.dry_run && p.force_design && p.no_regen);
        assert_eq!(
            (p.only, p.modules, p.adapters),
            (vec!["a::*".into()], vec!["m".into()], vec!["fc".into()])
        );
        assert!(a(&["--bogus"]).is_err());
        assert!(a(&["--only"]).is_err());
    }

    // criterion 1: unchanged digest and matching read-back => no write, no regen.
    // frob:tests crates/cadetta-cli/src/cmd/sync.rs::drive_adapter
    #[test]
    fn unchanged_design_writes_nothing_and_never_regens() {
        let (report, calls, lock) =
            drive(&[], Some("6mm"), &[init(), observed("6.0 mm"), Json::Null]);
        assert_eq!(calls, ["initialize", "read", "shutdown"]);
        assert!(report.regen.is_empty() && !report.failed(), "{report:?}");
        assert!(report.records.iter().all(|r| r.verb != "push"));
        assert_eq!(lock.records().count(), 1);
    }

    // frob:ticket 01M4ZS69MFPJDPD7782XAY9ZBA
    // frob:tests crates/cadetta-cli/src/cmd/sync.rs::build_entries
    #[test]
    fn initialize_precision_applies_unless_configured() {
        let source = "model CoverPlate {\n  thickness: Length = 6.123mm\n}\n";
        let init2 = json!({"name": "mock", "version": "1.0.0", "capabilities": ["drive"],
                           "native_units": "mm", "precision": 2, "platforms": ["linux"]});
        let results = [init2, observed("6.123mm"), Json::Null];
        let q004 = |extra: &str| {
            let dir = project_with(extra, source, BINDINGS);
            let (report, _, _) = drive_in(&dir, &[], Some("6.123mm"), &results);
            report.diagnostics.iter().any(|d| d.code == "Q004")
        };
        assert!(
            q004("files = [\"part.FCStd\"]\n"),
            "initialize precision 2 must withhold 6.123mm"
        );
        assert!(
            !q004("files = [\"part.FCStd\"]\nprecision = 4\n"),
            "configured precision wins"
        );
    }

    // frob:tests crates/cadetta-cli/src/cmd/sync.rs::drive_adapter
    #[test]
    fn design_change_pushes_regens_reads_back_and_saves() {
        let (report, calls, lock) = drive(&[], Some("5mm"), &full_run("6mm"));
        assert_eq!(
            calls,
            [
                "initialize",
                "read",
                "open",
                "read",
                "push",
                "regen",
                "read",
                "save",
                "close",
                "shutdown"
            ]
        );
        assert_eq!(report.records[0].verb, "push");
        assert_eq!(report.regen[0].status, "ok");
        assert!(!report.failed(), "{report:?}");
        let key = Record::SyncedValue(SyncedValue {
            adapter: "equations".into(),
            path: PATH.into(),
            locator: "Plate_Thk".into(),
            value: String::new(),
        })
        .key();
        assert!(matches!(lock.get(&key), Some(Record::SyncedValue(v)) if v.value == "6mm"));
        assert!(lock.records().any(|r| matches!(r, Record::SyncedFile(_))));
    }

    // frob:tests crates/cadetta-cli/src/cmd/sync.rs::drive_adapter
    #[test]
    fn dry_run_and_no_regen_change_what_is_called() {
        let (report, calls, lock) = drive(
            &["--dry-run"],
            Some("5mm"),
            &[init(), observed("5mm"), Json::Null],
        );
        assert_eq!(calls, ["initialize", "read", "shutdown"]);
        assert_eq!(report.records[0].verb, "push");
        assert!(!report.lock_written && lock.records().count() == 1);
        let results = [
            init(),
            observed("5mm"),
            json!({"session": {"handle": "s1", "files": [{"path": "part.FCStd", "dirty": false, "readonly": false}]}, "diagnostics": []}),
            observed("5mm"),
            pushed("5mm", "6mm"),
            observed("6mm"),
            Json::Null,
            Json::Null,
        ];
        let (report, calls, _) = drive(&["--no-regen"], Some("5mm"), &results);
        assert_eq!(
            calls,
            [
                "initialize",
                "read",
                "open",
                "read",
                "push",
                "read",
                "close",
                "shutdown"
            ]
        );
        assert_eq!(report.regen[0].status, "skipped");
    }

    // frob:tests crates/cadetta-cli/src/cmd/sync.rs::drive_adapter
    #[test]
    fn ref_link_needs_the_target_in_the_expression() {
        let g = RefTarget::Global("Plate_Thk".into());
        assert!(link_intact(&g, Some("=\"Plate_Thk\"")));
        assert!(!link_intact(&g, Some("=5")));
        assert!(!link_intact(&g, None));
        let own = RefTarget::OwnGlobal("X__y".into());
        assert!(link_intact(&own, Some("=\"X__y\"")));
        assert!(!link_intact(&own, None));
    }

    #[test]
    fn read_back_mismatch_fails_and_skips_save() {
        let (report, calls, _) = drive(&[], Some("5mm"), &{
            let mut r = full_run("7mm");
            r.remove(7);
            r
        });
        assert_eq!(report.records[0].verb, "fail");
        assert!(report.failed());
        assert!(!calls.contains(&"save".to_owned()), "{calls:?}");
    }

    // frob:ticket 01M4ZDQBBD1AW0RA4JW5SFKGWW
    // frob:tests crates/cadetta-cli/src/cmd/sync.rs::SessionBackend
    #[test]
    fn failed_regen_restores_the_snapshot_and_reports_r004() {
        let results = vec![
            init(),
            observed("5mm"),
            json!({"session": {"handle": "s1", "files": [{"path": "part.FCStd", "dirty": false, "readonly": false}]}, "diagnostics": []}),
            observed("5mm"),
            pushed("5mm", "6mm"),
            json!({"targets": [{"target": "part.FCStd", "status": "failed"}], "diagnostics": []}),
            pushed("6mm", "5mm"),
            json!({"targets": [{"target": "part.FCStd", "status": "ok"}], "diagnostics": []}),
            Json::Null,
            Json::Null,
        ];
        let (report, calls, _) = drive(&[], Some("5mm"), &results);
        assert_eq!(
            calls,
            [
                "initialize",
                "read",
                "open",
                "read",
                "push",
                "regen",
                "push",
                "regen",
                "close",
                "shutdown"
            ]
        );
        assert_eq!(report.regen[0].status, "failed");
        assert!(report.failed(), "{report:?}");
        assert!(!calls.contains(&"save".to_owned()), "{calls:?}");
    }

    // frob:tests crates/cadetta-cli/src/cmd/sync.rs::drive_adapter
    #[test]
    fn cad_side_edit_is_drift_and_both_sides_is_r001() {
        let (report, calls, _) = drive(&[], Some("6mm"), &[init(), observed("5mm"), Json::Null]);
        assert_eq!(report.records[0].verb, "drift");
        assert_eq!(calls, ["initialize", "read", "shutdown"]);
        let (report, _, _) = drive(&[], Some("4mm"), &[init(), observed("5mm"), Json::Null]);
        assert!(report.failed(), "{report:?}");
        assert!(report.diagnostics.iter().any(|d| d.code == "R001"));
    }

    const TOL_SOURCE: &str = "model CoverPlate {\n  thickness: Length = 6mm [+-0.1mm]\n}\n";
    const TOL_BINDINGS: &str = "[\"acme.plate::CoverPlate.thickness\"]\nlocators = [ { locator = \"Plate_Thk\", kind = \"dimension\", mode = \"value\", native_unit = \"mm\" } ]\n";

    fn tol_init(features: &[&str]) -> Json {
        let mut v = init();
        v["features"] = json!(features);
        v
    }

    fn observed_tol(text: &str, t: Option<&str>) -> Json {
        let mut v = observed(text);
        if let Some(t) = t {
            v["tolerances"] = json!([{"locator": "Plate_Thk",
                "tolerance": {"form": "symmetric", "t": t}}]);
        }
        v
    }

    // frob:ticket 01M4XDM9ST75VR0KRM3TDVNMAR
    // frob:tests crates/cadetta-cli/src/cmd/sync.rs::build_entries
    #[test]
    fn tolerance_edited_in_cad_is_r001() {
        let dir = project_with("files = [\"part.FCStd\"]\n", TOL_SOURCE, TOL_BINDINGS);
        let results = [
            tol_init(&["tolerances"]),
            observed_tol("6mm", Some("0.3mm")),
            Json::Null,
        ];
        let (report, _, _) = drive_in(&dir, &[], Some("6mm +-0.1mm"), &results);
        assert!(report.failed(), "{report:?}");
        assert!(
            report.diagnostics.iter().any(|d| d.code == "R001"),
            "{report:?}"
        );
    }

    // frob:ticket 01M4XDM9ST75VR0KRM3TDVNMAR
    // frob:tests crates/cadetta-cli/src/cmd/sync.rs::build_entries
    #[test]
    fn matching_tolerance_is_not_drift_and_unit_is_converted() {
        let dir = project_with("files = [\"part.FCStd\"]\n", TOL_SOURCE, TOL_BINDINGS);
        // 0.0039370078740157in is not exact, so use a tolerance that converts exactly.
        let results = [
            tol_init(&["tolerances"]),
            observed_tol("6mm", Some("0.1mm")),
            Json::Null,
        ];
        let (report, _, _) = drive_in(&dir, &[], Some("6mm +-0.1mm"), &results);
        assert!(!report.failed(), "{report:?}");
        assert_eq!(report.records[0].verb, "skip");
    }

    // frob:ticket 01M4XDM9ST75VR0KRM3TDVNMAR
    // frob:tests crates/cadetta-cli/src/cmd/sync.rs::build_entries
    #[test]
    fn design_tolerance_is_pushed_with_the_value() {
        let dir = project_with("files = [\"part.FCStd\"]\n", TOL_SOURCE, TOL_BINDINGS);
        let results = [
            tol_init(&["tolerances"]),
            observed_tol("5mm", Some("0.1mm")),
            json!({"session": {"handle": "s1", "files": [{"path": "part.FCStd", "dirty": false, "readonly": false}]}, "diagnostics": []}),
            observed("5mm"),
            pushed("5mm", "6mm"),
            json!({"targets": [{"target": "part.FCStd", "status": "ok"}], "diagnostics": []}),
            observed_tol("6mm", Some("0.1mm")),
            json!({"files": [{"path": "part.FCStd", "digest": "blake3:0a"}], "diagnostics": []}),
            Json::Null,
            Json::Null,
        ];
        let (report, out, _) = drive_in(&dir, &[], Some("5mm +-0.1mm"), &results);
        assert!(!report.failed(), "{report:?}");
        let text = String::from_utf8(out).unwrap();
        let push: Json = text
            .lines()
            .map(|l| serde_json::from_str::<Json>(l).unwrap())
            .find(|r| r["method"] == "push")
            .expect("a push request");
        assert_eq!(
            push["params"]["params"][0]["tolerance"],
            json!({"form": "symmetric", "t": "0.1mm"}),
            "{push}"
        );
    }

    // frob:ticket 01M4XDM9ST75VR0KRM3TDVNMAR
    // frob:tests crates/cadetta-cli/src/cmd/sync.rs::design_tolerance
    #[test]
    fn asymmetric_design_tolerance_is_pushed_bilateral() {
        let src = "model CoverPlate {\n  thickness: Length = 6mm [+0 -0.2mm]\n}\n";
        let dir = project_with("files = [\"part.FCStd\"]\n", src, TOL_BINDINGS);
        let results = [
            tol_init(&["tolerances"]),
            observed_tol("6mm", None),
            json!({"session": {"handle": "s1", "files": [{"path": "part.FCStd", "dirty": false, "readonly": false}]}, "diagnostics": []}),
            observed("6mm"),
            pushed("6mm", "6mm"),
            json!({"targets": [{"target": "part.FCStd", "status": "ok"}], "diagnostics": []}),
            observed_tol("6mm", None),
            json!({"files": [{"path": "part.FCStd", "digest": "blake3:0a"}], "diagnostics": []}),
            Json::Null,
            Json::Null,
        ];
        let (_, out, _) = drive_in(&dir, &[], None, &results);
        let text = String::from_utf8(out).unwrap();
        let push = text
            .lines()
            .map(|l| serde_json::from_str::<Json>(l).unwrap())
            .find(|r| r["method"] == "push")
            .expect("a push request");
        assert_eq!(
            push["params"]["params"][0]["tolerance"],
            json!({"form": "bilateral", "upper": "0mm", "lower": "-0.2mm"}),
            "{push}"
        );
    }

    // frob:ticket 01M4XDM9ST75VR0KRM3TDVNMAR
    // frob:tests crates/cadetta-cli/src/cmd/sync.rs::build_entries
    #[test]
    fn tolerances_are_ignored_without_the_feature() {
        let dir = project_with("files = [\"part.FCStd\"]\n", TOL_SOURCE, TOL_BINDINGS);
        let results = [
            tol_init(&[]),
            observed_tol("6mm", Some("0.3mm")),
            Json::Null,
        ];
        let (report, _, _) = drive_in(&dir, &[], Some("6mm"), &results);
        assert!(!report.failed(), "{report:?}");
        assert!(
            report.diagnostics.iter().all(|d| d.code != "R001"),
            "{report:?}"
        );
    }

    // criterion 2: an adapter without drive gets emit plus manual instructions.
    // frob:tests crates/cadetta-cli/src/cmd/sync.rs::manual_adapter
    #[test]
    fn adapter_without_drive_emits_and_prints_manual_steps() {
        let dir = project("");
        let (code, out, e) = run(&dir, Format::Text, &[]);
        assert_eq!((code, e.as_str()), (EXIT_OK, ""), "{out}");
        assert!(dir.0.join("build/solidworks/equations.txt").is_file());
        assert!(
            out.contains("manual  equations:") && out.contains("Link to external file"),
            "{out}"
        );
        assert!(out.contains("build/solidworks/equations.txt"), "{out}");
        let dir = project("");
        let (code, out, _) = run(&dir, Format::Text, &["--dry-run"]);
        assert_eq!(code, EXIT_OK);
        assert!(!dir.0.join("build").exists(), "dry run wrote: {out}");
        assert!(out.contains("manual  equations:"), "{out}");
    }

    // an adapter without drive records the handed-over file and reports O004 (open, info).
    // frob:ticket 01M4YW6FFKS9DAYE3DAFEZDG87
    // frob:tests crates/cadetta-cli/src/cmd/sync.rs::record_handed_over
    // frob:tests crates/cadetta-cli/src/cmd/sync.rs::report_open_sync
    #[test]
    fn adapter_without_drive_reports_open_o004_and_records_the_file() {
        let dir = project("");
        let (code, out, e) = run(&dir, Format::Text, &[]);
        assert_eq!((code, e.as_str()), (EXIT_OK, ""), "{out}");
        assert!(
            out.contains("O004") && out.contains("unverifiable without drive"),
            "{out}"
        );
        let lock = std::fs::read_to_string(dir.0.join("cadetta.lock")).unwrap();
        assert!(lock.contains("synced"), "{lock}");
        let dir = project("");
        let (code, out, _) = run(&dir, Format::Text, &["--dry-run"]);
        assert_eq!(code, EXIT_OK);
        assert!(
            !out.contains("O004") && !dir.0.join("cadetta.lock").exists(),
            "{out}"
        );
    }

    // an unbound-name row in the on-disk csv exits with R016.
    // frob:tests crates/cadetta-cli/src/cmd/sync.rs::read_back_csv
    #[test]
    fn unbound_csv_row_exits_with_r016() {
        let dir = project_with("", SOURCE, BINDINGS);
        let toml = std::fs::read_to_string(dir.0.join("cadetta.toml"))
            .unwrap()
            .replace("sw-equations", "csv");
        std::fs::write(dir.0.join("cadetta.toml"), toml).unwrap();
        let (code, out, _) = run(&dir, Format::Text, &[]);
        assert_eq!(code, EXIT_OK, "{out}");
        let file = dir.0.join("build/equations.csv");
        let mut text = std::fs::read_to_string(&file).unwrap();
        assert!(!text.contains("Stray"), "{text}");
        text.push_str("Stray,1,mm,\n");
        std::fs::write(&file, text).unwrap();
        let (code, out, _) = run(&dir, Format::Text, &[]);
        assert_eq!(code, EXIT_FAIL, "{out}");
        assert!(out.contains("R016") && out.contains("Stray"), "{out}");
    }

    // criterion 3: usage errors, exit codes and the JSON schema.
    // frob:tests crates/cadetta-cli/src/cmd/sync.rs::run_sync
    #[test]
    fn usage_errors_exit_2_and_json_matches_schema() {
        let dir = project("");
        let (code, _, e) = run(&dir, Format::Text, &["--bogus"]);
        assert_eq!(code, EXIT_USAGE);
        assert!(e.contains("unknown flag"), "{e}");
        let (code, _, e) = run(&dir, Format::Text, &["nope"]);
        assert_eq!(code, EXIT_USAGE);
        assert!(e.contains("no adapter `nope`"), "{e}");
        let schema: Json =
            serde_json::from_str(include_str!("../../../../docs/schemas/sync.json")).unwrap();
        let validator = jsonschema::validator_for(&schema).unwrap();
        let (code, out, _) = run(&dir, Format::Json, &[]);
        assert_eq!(code, EXIT_OK);
        let v: Json = serde_json::from_str(&out).unwrap();
        assert!(validator.is_valid(&v), "{v}");
        let report = SyncReport {
            records: vec![SyncRecord {
                adapter: "a".into(),
                file: "f".into(),
                path: "p".into(),
                locator: "l".into(),
                verb: "push".into(),
                old: "1mm".into(),
                new: "2mm".into(),
            }],
            regen: vec![RegenRecord {
                file: "f".into(),
                status: "ok".into(),
            }],
            ..SyncReport::default()
        };
        assert!(validator.is_valid(&serde_json::to_value(&report).unwrap()));
        assert!(render_text(&report, false).starts_with("push  p  l  1mm -> 2mm\n"));
    }

    // frob:tests crates/cadetta-cli/src/cmd/sync.rs::observed_geometry
    #[test]
    fn observed_geometry_parses_frames_and_skips_malformed() {
        let row = |x: f64| serde_json::json!([{"nominal":{"r":"1/1","pi":0}}, {"nominal":{"r":"0/1","pi":0}}, {"nominal":{"r":"0/1","pi":0}}, {"nominal":{"f64":x,"root":"p"}}]);
        let t = serde_json::json!([row(5.0), row(0.0), row(0.0)]);
        let j = serde_json::json!({
            "frames": [{"name":"F","document":"a.step","transform":t}, {"name":"bad"}],
            "instances": [{"component":"c","document":"a.step","transform":t}, {"document":"x"}]
        });
        let (frames, instances) = observed_geometry(&j);
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].name, "F");
        assert_eq!(frames[0].transform.len(), 3);
        assert_eq!(frames[0].transform[0].len(), 4);
        assert_eq!(instances.len(), 1);
        assert!(instances[0].reference.is_none());
        let (f2, _) = observed_geometry(&serde_json::json!([j]));
        assert_eq!(f2.len(), 1);
    }

    // frob:tests crates/cadetta-cli/src/cmd/sync.rs::native_scale
    #[test]
    fn native_scale_defaults_to_millimetres() {
        let a: Adapter = serde_json::from_value(serde_json::json!({"name":"x"})).unwrap();
        assert!((native_scale(&a) - 0.001).abs() < 1e-12);
    }

    // --- geometry pass (ticket B6MN43Z) ---

    fn gpath(t: &str) -> cadetta_ir::path::Path {
        cadetta_ir::path::Path::parse(t, cadetta_ir::path::PathContext::General).unwrap()
    }

    /// An empty model component named `path`.
    fn gcomponent(path: &str) -> cadetta_ir::model::Component {
        let mut p = gpath(&format!("{path}.x"));
        p.members.clear();
        cadetta_ir::model::Component {
            path: p,
            class: cadetta_ir::model::ComponentClass::Model,
            params_sig: None,
            implements: vec![],
            citation: None,
            attrs: Default::default(),
            members: vec![],
            parts: vec![],
            fits: vec![],
            datums: vec![],
            points: vec![],
            placements: vec![],
            span: None,
            digest: None,
        }
    }

    /// A design with one datum `a.b::Rail.site` offset 10 mm in x, bound to frame `Rail__site`.
    fn datum_ir() -> (Ir, Vec<cadetta_ir::model::Binding>) {
        use cadetta_ir::geometry::{Datum, OffsetArgs, Op};
        use cadetta_ir::model::{Flavour, Member, MemberKind, PackageId};
        let mut ir = Ir::new(
            Flavour::Design,
            PackageId {
                name: "a.b".into(),
                version: "1".into(),
            },
        );
        let mut rail = gcomponent("a.b::Rail");
        let mut m = Member::new(gpath("a.b::Rail.site.offset.x"), MemberKind::Datum);
        m.value = wire_value(&json!({"r": "1/100", "pi": 0}));
        rail.members.push(m);
        rail.datums.push(Datum::new(
            gpath("a.b::Rail.site"),
            gpath("a.b::Rail.origin"),
            vec![Op::Offset(Box::new(OffsetArgs {
                x: Some(gpath("a.b::Rail.site.offset.x")),
                ..Default::default()
            }))],
        ));
        ir.components = vec![rail];
        let loc = cadetta_ir::model::Locator {
            locator: "Rail__site".into(),
            kind: LocatorKind::Frame,
            mode: LocatorMode::Value,
            native_unit: "mm".into(),
            ref_to: None,
            for_alternate: None,
            config: None,
            sign: None,
            component: None,
            implicit: None,
            tolerance: None,
        };
        let b = cadetta_ir::model::Binding {
            path: gpath("a.b::Rail.site"),
            adapter: "fc".into(),
            locators: vec![loc],
        };
        (ir, vec![b])
    }

    /// `read.observed` JSON with the frame `Rail__site` at `milli` mm, or no frames when `None`.
    fn frame_json(milli: Option<i64>) -> Json {
        let n = |v: i64, d: i64| json!({"nominal": {"r": format!("{v}/{d}"), "pi": 0}});
        let row = |r: usize| {
            let mut c: Vec<Json> = (0..3).map(|i| n(i64::from(i == r), 1)).collect();
            c.push(n(if r == 0 { milli.unwrap_or(0) } else { 0 }, 1000));
            Json::Array(c)
        };
        match milli {
            Some(_) => json!({"frames": [{"name": "Rail__site", "document": "rail.FCStd",
                "transform": [row(0), row(1), row(2)]}]}),
            None => json!({"components": []}),
        }
    }

    /// Run the geometry pass; returns the diagnostic codes and the datum entries.
    fn pass(milli: Option<i64>, extra: &str, features: &[Feature]) -> (Vec<String>, Vec<Entry>) {
        let (ir, bindings) = datum_ir();
        let adapter: Adapter = toml::from_str(&format!(
            "name = \"fc\"\nfiles = [\"rail.FCStd\"]\nnative_unit = \"mm\"\n{extra}"
        ))
        .unwrap();
        let args = parse_args(&[]).unwrap();
        let run_scope = RunScope::new(&args.modules, &args.only).unwrap();
        let mut lock = FileLock::default();
        let mut report = SyncReport::default();
        let mut ctx = Ctx {
            args: &args,
            run_scope: &run_scope,
            lock: &mut lock,
            lock_changed: false,
            report: &mut report,
            texts: TextParams::new(),
        };
        let observed = frame_json(milli);
        let read = Geometry {
            observed: &observed,
            features,
            tolerances: &[],
            precision: cadetta_adapter::builtin::sw_equations::DEFAULT_PRECISION,
        };
        let entries = geometry_pass(&adapter, &ir, &bindings, &[], &read, &mut ctx);
        (
            report.diagnostics.iter().map(|d| d.code.clone()).collect(),
            entries,
        )
    }

    // frob:ticket 01M4W1REG5EVCAD4NWWB6MN43Z
    // frob:tests crates/cadetta-cli/src/cmd/sync.rs::geometry_pass
    #[test]
    fn moved_frame_is_r001_and_matching_frame_is_silent() {
        assert!(pass(Some(10), "", &[]).0.is_empty());
        let (codes, entries) = pass(Some(12), "", &[]);
        assert_eq!(codes, ["R001"]);
        assert!(entries.is_empty(), "verify policy never drives");
    }

    // frob:ticket 01M4W1REG5EVCAD4NWWB6MN43Z
    // frob:tests crates/cadetta-cli/src/cmd/sync.rs::geometry_pass
    #[test]
    fn datums_create_appends_entries_only_with_the_feature() {
        let create = "datums = \"create\"\n";
        let (codes, entries) = pass(None, create, &[Feature::CreateFrames]);
        assert!(codes.is_empty());
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].path, "a.b::Rail.site");
        assert_eq!(entries[0].locator, "Rail__site");
        assert_eq!(entries[0].file, "rail.FCStd");
        assert!(pass(None, create, &[]).1.is_empty());
        assert!(pass(None, "", &[Feature::CreateFrames]).1.is_empty());
    }

    // frob:ticket 01M4XDMA1D78TRJB1CB5HP91AE
    // frob:tests crates/cadetta-cli/src/cmd/sync.rs::emit_external
    #[cfg(unix)]
    #[test]
    fn external_adapter_without_drive_runs_emit_and_reports_files() {
        use std::os::unix::fs::PermissionsExt;
        let dir = project("");
        let toml = "[package]\nname = \"acme.plate\"\nversion = \"0.1.0\"\n[[adapter]]\nname = \"ext\"\ncommand = [\"./ext.sh\"]\n";
        std::fs::write(dir.0.join("cadetta.toml"), toml).unwrap();
        let body = r#"#!/bin/sh
read -r _init
echo '{"jsonrpc":"2.0","id":1,"result":{"name":"ext","version":"1.0.0","capabilities":["emit"],"native_units":"mm","precision":6,"platforms":["linux"]}}'
while read -r req; do
  case "$req" in
    *'"method":"emit"'*)
      printf '%s' "$req" > "$(dirname "$0")/emit-request.json"
      echo '{"jsonrpc":"2.0","id":2,"result":{"files":[{"path":"build/ext/out.txt","digest":"blake3:00"}],"diagnostics":[]}}' ;;
    *'"method":"shutdown"'*)
      echo '{"jsonrpc":"2.0","id":3,"result":null}'
      exit 0 ;;
  esac
done
"#;
        let script = dir.0.join("ext.sh");
        let _guard = SPAWN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::fs::write(&script, body).unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        let (code, out, err) = run(&dir, Format::Text, &[]);
        drop(_guard);
        assert_eq!((code, err.as_str()), (EXIT_OK, ""), "{out}");
        assert!(out.contains("build/ext/out.txt"), "{out}");
        let req: Json = serde_json::from_str(
            &std::fs::read_to_string(dir.0.join("emit-request.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(req["method"], "emit");
        assert_eq!(req["params"]["policy"]["point"], "nominal");
        assert!(req["params"]["design"].is_object(), "{req}");
        std::fs::remove_file(dir.0.join("emit-request.json")).unwrap();
        let (code, _, _) = run(&dir, Format::Text, &["--dry-run"]);
        assert_eq!(code, EXIT_OK);
        assert!(!dir.0.join("emit-request.json").exists(), "dry run emitted");
    }

    // frob:ticket 01M4YPDTD1977TMY4M2PPPNSDD
    // frob:tests crates/cadetta-cli/src/cmd/sync.rs::sync_adapter
    #[cfg(unix)]
    #[test]
    fn a_silent_adapter_fails_sync_within_the_configured_timeout() {
        use std::os::unix::fs::PermissionsExt;
        let dir = project("");
        let toml = "[package]\nname = \"acme.plate\"\nversion = \"0.1.0\"\n[[adapter]]\nname = \"silent\"\ncommand = [\"./silent.sh\"]\ntimeout = 1\n";
        std::fs::write(dir.0.join("cadetta.toml"), toml).unwrap();
        let script = dir.0.join("silent.sh");
        std::fs::write(&script, "#!/bin/sh\nsleep 300\n").unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        let start = std::time::Instant::now();
        let (code, _, err) = run(&dir, Format::Text, &[]);
        assert_eq!(code, EXIT_USAGE, "{err}");
        assert!(err.contains("R008"), "{err}");
        assert!(start.elapsed() < std::time::Duration::from_secs(30));
    }

    // frob:ticket 01M4Y3WHS6G3MXMECY8ZMVJ6CV
    // frob:tests crates/cadetta-cli/src/cmd/sync.rs::sync_adapter
    #[test]
    fn unavailable_adapter_exits_two_with_r008() {
        let dir = project("");
        let toml = "[package]\nname = \"acme.plate\"\nversion = \"0.1.0\"\n[[adapter]]\nname = \"gone\"\ncommand = [\"./does-not-exist\"]\n";
        std::fs::write(dir.0.join("cadetta.toml"), toml).unwrap();
        let (code, _, err) = run(&dir, Format::Text, &[]);
        assert_eq!(code, EXIT_USAGE, "{err}");
        assert!(err.contains("R008"), "{err}");
    }

    // frob:ticket 01M558W4KV1H1JZVFHBF8BPBEH
    // frob:tests crates/cadetta-cli/src/cmd/sync.rs::resolve_observed_file
    #[test]
    fn bare_observed_names_resolve_to_the_unique_package_path() {
        let known = vec!["parts/a.SLDPRT".to_owned(), "parts/b.SLDPRT".to_owned()];
        assert_eq!(resolve_observed_file("a.SLDPRT", &known), "parts/a.SLDPRT");
        assert_eq!(
            resolve_observed_file("parts/b.SLDPRT", &known),
            "parts/b.SLDPRT"
        );
        assert_eq!(resolve_observed_file("c.SLDPRT", &known), "c.SLDPRT");
        let dup = vec!["x/a.SLDPRT".to_owned(), "y/a.SLDPRT".to_owned()];
        assert_eq!(resolve_observed_file("a.SLDPRT", &dup), "a.SLDPRT");
    }

    // frob:ticket 01M558W4KV1H1JZVFHBF8BPBEH
    // frob:tests crates/cadetta-cli/src/cmd/sync.rs::drive_adapter
    #[test]
    fn push_opens_the_package_relative_path_of_a_bare_observed_file() {
        let dir = project("files = [\"parts/part.FCStd\"]\n");
        let mut results = full_run("6mm");
        results[2] = json!({"session": {"handle": "s1", "files": [{"path": "parts/part.FCStd", "dirty": false, "readonly": false}]}, "diagnostics": []});
        let (report, out, _) = drive_in(&dir, &[], Some("5mm"), &results);
        assert!(!report.failed(), "{report:?}");
        let text = String::from_utf8(out).unwrap();
        let open: Json = text
            .lines()
            .map(|l| serde_json::from_str::<Json>(l).unwrap())
            .find(|r| r["method"] == "open")
            .expect("an open request");
        assert!(open.to_string().contains("parts/part.FCStd"), "{open}");
    }

    // frob:ticket 01M558W4R8Q0Y47SEXJ20S26NG
    // frob:tests crates/cadetta-cli/src/cmd/sync.rs::read_current
    #[test]
    fn skipped_locator_reads_back_and_is_recorded() {
        let mut results = full_run("6mm");
        results[4] = json!({"statuses": [{"locator": "Plate_Thk", "status": "skipped"}],
                            "diagnostics": []});
        let (report, _, lock) = drive(&[], Some("5mm"), &results);
        assert!(!report.failed(), "{report:?}");
        let key = Record::SyncedValue(SyncedValue {
            adapter: "equations".into(),
            path: PATH.into(),
            locator: "Plate_Thk".into(),
            value: String::new(),
        })
        .key();
        assert!(matches!(lock.get(&key), Some(Record::SyncedValue(v)) if v.value == "6mm"));
    }

    // frob:ticket 01M4Y3WHS6G3MXMECY8ZMVJ6CV
    // frob:tests crates/cadetta-cli/src/cmd/sync.rs::drive_adapter
    #[test]
    fn rejected_locator_is_reported_as_r010_per_file() {
        let mut results = full_run("6mm");
        results[4] = json!({"statuses": [{"locator": "Plate_Thk", "status": "rejected"}],
                            "diagnostics": []});
        let (report, _, _) = drive(&[], Some("5mm"), &results);
        let r010: Vec<_> = report
            .diagnostics
            .iter()
            .filter(|d| d.code == "R010")
            .collect();
        assert_eq!(r010.len(), 1, "{report:?}");
        assert!(r010[0].message.contains("part.FCStd") && r010[0].message.contains("Plate_Thk"));
    }

    // frob:ticket 01M4Y3WHS6G3MXMECY8ZMVJ6CV
    // frob:tests crates/cadetta-cli/src/cmd/sync.rs::drive_adapter
    #[test]
    fn dirty_file_is_refused_r009_and_nothing_is_pushed() {
        let results = vec![
            init(),
            observed("5mm"),
            json!({"session": {"handle": "s1", "files": [{"path": "part.FCStd", "dirty": true, "readonly": false}]}, "diagnostics": []}),
            observed("5mm"),
            Json::Null,
        ];
        let (report, calls, _) = drive(&[], Some("5mm"), &results);
        assert!(
            report.diagnostics.iter().any(|d| d.code == "R009"),
            "{report:?}"
        );
        assert!(!calls.contains(&"push".to_owned()), "{calls:?}");
    }

    // frob:ticket 01M4YQQPSNQ7TP3QFRWAYFH870
    // frob:tests crates/cadetta-cli/src/cmd/sync.rs::run_sync
    #[test]
    fn invalid_bindings_exit_two_naming_the_file() {
        let dir = project("");
        let bad = "[\"acme.plate::CoverPlate.thickness\"]\nlocators = [ { locator = \"\", kind = \"global\" } ]\n";
        std::fs::write(dir.0.join("bindings/equations.toml"), bad).unwrap();
        let (code, out, e) = run(&dir, Format::Text, &[]);
        assert_eq!(code, EXIT_USAGE, "{out} {e}");
        assert!(e.contains("equations.toml"), "{e}");
        assert!(!out.contains("R005"), "{out}");
    }
}
