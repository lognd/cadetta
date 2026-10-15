//! frob:ticket 01M4VQXM0V12H5JBPHWW796HD7
//! Executors for the `binding` entries `read` and `emit-read`: they drive the adapter host's
//! real read-side code (native-link discovery, sync's R003, name qualification, unit
//! resolution, the built-in csv/json/sw-equations readers and the round-trip contract check)
//! on the scenario the fixture JSON describes and record what the expectation observes.

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use cadetta_adapter::bindings::{ForwardTable, NameSource, Placement, discover, reconcile_read};
use cadetta_adapter::builtin::{csv, json, sw_equations};
use cadetta_adapter::locator::check_native_collisions;
use cadetta_adapter::read_facts;
use cadetta_adapter::read_units::{NativeValue, ObservedValue, UnitPolicy};
use cadetta_adapter::sync::{self, Lock};
use cadetta_adapter::testing::{Observed, ReadAdapter, check_round_trip};
use cadetta_adapter::{
    Adapter, AdapterDiagnostic, AdapterError, AdapterResult, Capabilities, Capability, Outcome,
    Severity,
};
use cadetta_ir::model::{
    Binding, Exact, Locator, LocatorKind, LocatorMode, MemberKind, Nominal, ObservedFrame,
    ObservedInstance, PackageId, Value as IrValue,
};
use cadetta_ir::observed::{ObservedBuilder, ObservedParam};
use cadetta_ir::path::{Path, PathContext};
use cadetta_units::{Dim, Magnitude, Rational};
use serde_json::{Map, Value, json};

use crate::compare::Produced;
use crate::exec::Job;
use crate::run::{Facts, Scratch};

fn spec_of(job: &Job) -> Result<Value, String> {
    let text =
        fs::read_to_string(job.fixture).map_err(|e| format!("{}: {e}", job.fixture.display()))?;
    serde_json::from_str(&text).map_err(|e| format!("{}: {e}", job.fixture.display()))
}

fn strs(spec: &Value, key: &str) -> Vec<String> {
    spec[key]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|v| v.as_str().map(str::to_owned))
        .collect()
}

fn path(text: &str) -> Result<Path, String> {
    Path::parse(text, PathContext::Binding).map_err(|e| format!("path `{text}`: {e}"))
}

fn severity_word(s: Severity) -> &'static str {
    match s {
        Severity::Info => "info",
        Severity::Warning => "warning",
        Severity::Error => "error",
    }
}

/// Facts carrying `produced`; the exit is 1 when any diagnostic is an error.
fn facts_of(produced: Vec<Produced>) -> Facts {
    let exit = i32::from(produced.iter().any(|p| p.severity == "error"));
    Facts {
        produced,
        exit,
        ..Facts::default()
    }
}

fn produced_adapter(d: &AdapterDiagnostic) -> Produced {
    Produced {
        code: d.code.clone(),
        severity: severity_word(d.severity).to_owned(),
        path: None,
    }
}

/// `read`: dispatch on the scenario shape the fixture describes.
pub fn read(job: &Job) -> Result<Facts, String> {
    let spec = spec_of(job)?;
    let has = |k: &str| spec.get(k).is_some();
    tracing::debug!(fixture = %job.fixture.display(), "conformance: binding read");
    if has("native_globals") {
        native_link(&spec)
    } else if has("implicit_synced") {
        vanished(&spec)
    } else if has("locators") && spec["locators"][0].is_object() {
        qualified(&spec)
    } else if has("measured_from") {
        measured_from(&spec)
    } else if has("lock_digest") {
        lock_digest(&spec)
    } else if has("slot_primary_file") {
        slot_files(&spec)
    } else if has("native_names") {
        reconciled(&spec)
    } else if has("native") {
        unit_read(&spec)
    } else if has("text") && has("file") {
        emitter_read(&spec)
    } else {
        Err("this read scenario has no adapter-host behaviour wired yet".into())
    }
}

/// The package the native-link scenarios read: `Plate.t` (once `Plate.thickness`) and a datum.
fn forward_table() -> Result<ForwardTable, String> {
    let src = |p: &str, was: &[&str], kind| -> Result<NameSource, String> {
        Ok(NameSource {
            path: path(p)?,
            was: was.iter().map(|s| (*s).to_owned()).collect(),
            kind,
        })
    };
    ForwardTable::build(
        &[
            src(
                "acme.demo::Plate.t",
                &["acme.demo::Plate.thickness"],
                MemberKind::Param,
            )?,
            src("acme.demo::Plate.origin", &[], MemberKind::Datum)?,
        ],
        "acme_demo__",
    )
    .map_err(|e| format!("forward table: {e}"))
}

/// A 3x4 identity transform (rotation columns then zero translation).
fn identity() -> Vec<Vec<IrValue>> {
    (0..3)
        .map(|r| {
            (0..4)
                .map(|c| {
                    let n = i64::from(r == c);
                    IrValue::new(
                        Nominal::Exact(Exact(Magnitude::new(Rational::new(n, 1).expect("n/1"), 0))),
                        Dim::default(),
                        n.to_string(),
                    )
                })
                .collect()
        })
        .collect()
}

fn locator(name: &str, kind: LocatorKind) -> Locator {
    Locator {
        locator: name.into(),
        kind,
        mode: LocatorMode::Value,
        native_unit: "mm".into(),
        ref_to: None,
        for_alternate: None,
        config: None,
        sign: None,
        component: None,
        implicit: None,
        tolerance: None,
    }
}

fn native_param(name: &str, kind: LocatorKind, expr: Option<&str>) -> ObservedParam {
    ObservedParam {
        native_name: name.into(),
        locator: locator(name, kind),
        nominal: Nominal::Exact(Exact(Magnitude::new(Rational::new(5, 1).expect("5/1"), 0))),
        dim: Dim::default(),
        text: "5".into(),
        expr: expr.map(str::to_owned),
        unit_assumed: Some("mm".into()),
        inexact_reason: None,
    }
}

/// Native-link discovery: the fixture's native items go through `bindings::discover`.
fn native_link(spec: &Value) -> Result<Facts, String> {
    let pkg = PackageId {
        name: "acme.demo".into(),
        version: "0.0.0".into(),
    };
    let mut b = ObservedBuilder::new(pkg, "fixture", "f.sldprt").map_err(|e| e.to_string())?;
    let mut push = |p: ObservedParam| b.push(p).map_err(|e| e.to_string());
    for g in strs(spec, "native_globals") {
        push(native_param(&g, LocatorKind::Global, None))?;
    }
    for (i, d) in spec["native_dimensions"]
        .as_array()
        .into_iter()
        .flatten()
        .enumerate()
    {
        let name = d["name"]
            .as_str()
            .map_or_else(|| format!("D{i}@Boss"), str::to_owned);
        push(native_param(
            &name,
            LocatorKind::Dimension,
            d["equation"].as_str(),
        ))?;
    }
    for f in strs(spec, "native_frames") {
        b.push_frame(ObservedFrame {
            name: f,
            document: "f".into(),
            transform: identity(),
        })
        .map_err(|e| e.to_string())?;
    }
    for i in spec["native_instances"].as_array().into_iter().flatten() {
        b.push_instance(ObservedInstance {
            component: i["name"].as_str().unwrap_or_default().into(),
            document: "asm".into(),
            reference: i["reference"].as_str().map(str::to_owned),
            transform: identity(),
        })
        .map_err(|e| e.to_string())?;
    }
    let placements = [Placement {
        reference: "bracket".into(),
        node: path("acme.demo::Mount/bracket.place")?,
    }];
    let found = discover(&b.build(), &forward_table()?, &placements);
    tracing::info!(
        bindings = found.bindings.len(),
        diagnostics = found.diagnostics.len(),
        "conformance: native-link discovery"
    );
    Ok(facts_of(
        found.diagnostics.iter().map(produced_adapter).collect(),
    ))
}

/// A recorded implicit locator that the read no longer finds is R003 (`sync::missing_implicit`).
fn vanished(spec: &Value) -> Result<Facts, String> {
    let path_text = "acme.demo::Plate.t".to_owned();
    let synced = strs(spec, "implicit_synced");
    let names: BTreeSet<String> = strs(spec, "native_names").into_iter().collect();
    let implicit: Vec<(String, String)> = synced
        .iter()
        .map(|l| (path_text.clone(), l.clone()))
        .collect();
    let lock: Lock = synced
        .iter()
        .map(|l| {
            (
                ("fixture".to_owned(), path_text.clone(), l.clone()),
                "10mm".to_owned(),
            )
        })
        .collect();
    let found: BTreeSet<(String, String)> =
        names.into_iter().map(|l| (path_text.clone(), l)).collect();
    let diags = sync::missing_implicit("fixture", &implicit, &found, &lock);
    Ok(facts_of(
        diags
            .into_iter()
            .map(|d| Produced {
                code: d.code.to_owned(),
                severity: "error".to_owned(),
                path: None,
            })
            .collect(),
    ))
}

/// Native names map through bindings only: unbound is R002, a bound but absent one is R003.
fn reconciled(spec: &Value) -> Result<Facts, String> {
    let mut bound = Vec::new();
    for l in strs(spec, "locators") {
        bound.push((path(&format!("acme.demo::Plate.{l}"))?, l));
    }
    for (name, target) in spec["bindings"].as_object().into_iter().flatten() {
        bound.push((path(target.as_str().unwrap_or_default())?, name.clone()));
    }
    let found = reconcile_read(&bound, &strs(spec, "native_names"));
    let mapped: Map<String, Value> = found
        .mapped
        .iter()
        .map(|(n, p)| (n.clone(), json!(p.to_string())))
        .collect();
    let mut facts = facts_of(found.diagnostics.iter().map(produced_adapter).collect());
    facts
        .observed
        .insert("mapped".into(), Value::Object(mapped));
    Ok(facts)
}

/// R007: a dimension locator whose `measured_from` is not the sketch origin.
fn measured_from(spec: &Value) -> Result<Facts, String> {
    let kind = kind_of(spec["locator"]["kind"].as_str().unwrap_or("global"))?;
    let from = spec["measured_from"].as_str().unwrap_or_default();
    let found = read_facts::check_measured_from(kind, from);
    Ok(facts_of(found.iter().map(produced_adapter).collect()))
}

fn string_map(v: &Value) -> std::collections::BTreeMap<String, String> {
    v.as_object()
        .into_iter()
        .flatten()
        .map(|(k, x)| (k.clone(), x.as_str().unwrap_or_default().to_owned()))
        .collect()
}

/// An unchanged saved file (equal digest) is read from the lock without opening the tool.
fn lock_digest(spec: &Value) -> Result<Facts, String> {
    let values = string_map(&spec["lock_values"]);
    let reused = read_facts::reusable_values(
        spec["on_disk_digest"].as_str().unwrap_or_default(),
        spec["lock_digest"].as_str().unwrap_or_default(),
        &values,
    );
    let mut facts = facts_of(Vec::new());
    facts
        .observed
        .insert("opened_in_tool".into(), json!(reused.is_none()));
    if let Some(v) = reused {
        facts.observed.insert("observed_values".into(), json!(v));
    }
    Ok(facts)
}

/// A slots read records each component instance's file; R011 when it is not the slot's file.
fn slot_files(spec: &Value) -> Result<Facts, String> {
    let primary = spec["slot_primary_file"].as_str().unwrap_or_default();
    let mut instances = string_map(&spec["instances"]);
    if let Some(f) = spec["assembly_instance_file"].as_str() {
        instances.insert("instance".into(), f.to_owned());
    }
    let (seen, diags) = read_facts::observe_instance_files(primary, &instances);
    let mut facts = facts_of(diags.iter().map(produced_adapter).collect());
    if spec.get("instances").is_some() {
        facts
            .observed
            .insert("observed_instance_files".into(), json!(seen));
    }
    Ok(facts)
}

fn kind_of(word: &str) -> Result<LocatorKind, String> {
    match word {
        "global" => Ok(LocatorKind::Global),
        "dimension" => Ok(LocatorKind::Dimension),
        "property" => Ok(LocatorKind::Property),
        "frame" => Ok(LocatorKind::Frame),
        "instance" => Ok(LocatorKind::Instance),
        other => Err(format!("unknown locator kind `{other}`")),
    }
}

/// Native names are qualified by kind; one name for two paths in one namespace is R013.
fn qualified(spec: &Value) -> Result<Facts, String> {
    let mut bindings = Vec::new();
    for l in spec["locators"].as_array().into_iter().flatten() {
        bindings.push(Binding {
            path: path(&format!(
                "acme.demo::Plate.{}",
                l["path"].as_str().unwrap_or("a")
            ))?,
            adapter: "fixture".into(),
            locators: vec![locator(
                l["locator"].as_str().unwrap_or_default(),
                kind_of(l["kind"].as_str().unwrap_or("global"))?,
            )],
        });
    }
    let mut names: Vec<String> = bindings
        .iter()
        .flat_map(|b| {
            b.locators
                .iter()
                .map(cadetta_adapter::locator::qualified_name)
        })
        .collect();
    names.sort();
    names.dedup();
    let mut facts = match check_native_collisions(&bindings) {
        Ok(()) => facts_of(Vec::new()),
        Err(AdapterError::Failed(ds)) => facts_of(ds.iter().map(produced_adapter).collect()),
        Err(e) => return Err(format!("collision check: {e}")),
    };
    facts.observed.insert("qualified".into(), json!(names));
    Ok(facts)
}

/// `value` text of an observed value: literal then unit.
fn value_text(o: &ObservedValue) -> String {
    format!("{}{}", o.literal, o.unit.as_deref().unwrap_or_default())
}

fn policy_of(native: Option<&str>, assume: Option<&str>) -> UnitPolicy {
    UnitPolicy {
        native_unit: native.map(str::to_owned),
        assume_unit: assume.map(str::to_owned),
    }
}

/// One native number read through the adapter's unit policy (`--assume-unit` from `args`).
fn unit_read(spec: &Value) -> Result<Facts, String> {
    let args = strs(spec, "args");
    let assume = args
        .iter()
        .position(|a| a == "--assume-unit")
        .and_then(|i| args.get(i + 1))
        .map(String::as_str);
    let units = policy_of(spec["adapter"]["native_unit"].as_str(), assume);
    let native = &spec["native"];
    let number = native["number"].to_string();
    let mut o = units.read(&NativeValue {
        literal: number,
        unit: native["unit"].as_str().map(str::to_owned),
    });
    let mut out = Map::new();
    if let Some(expr) = native["expression"].as_str() {
        let line = format!(
            "\"{}\" = {expr}",
            spec["locator"]["locator"].as_str().unwrap_or("X")
        );
        match sw_equations::read(&line, &units).first().map(|l| &l.rhs) {
            Some(sw_equations::ReadRhs::Expr(_)) => {
                out.insert("expression".into(), json!(expr));
            }
            _ => {
                o.exact = false;
                o.reason = Some(format!("expression `{expr}` is not parseable"));
            }
        }
    }
    out.insert("value".into(), json!(value_text(&o)));
    out.insert("unit_assumed".into(), json!(o.unit_assumed));
    out.insert("exact".into(), json!(o.exact));
    out.insert("reason_present".into(), json!(o.reason.is_some()));
    let mut facts = facts_of(Vec::new());
    facts.observed.insert("observed".into(), Value::Object(out));
    Ok(facts)
}

/// The per-parameter record shared by every emitter read.
fn record(kind: &str, o: &ObservedValue, path: Option<&str>) -> Map<String, Value> {
    let mut m = Map::new();
    m.insert("kind".into(), json!(kind));
    m.insert("value".into(), json!(value_text(o)));
    if o.unit_assumed
        && let Some(u) = &o.unit
    {
        m.insert("unit_assumed".into(), json!(u));
    }
    if let Some(p) = path {
        m.insert("path".into(), json!(p));
    }
    m.insert("exact".into(), json!(o.exact));
    m
}

/// Read a csv/json/sw-equations file (chosen by the fixture's `file` extension).
fn emitter_read(spec: &Value) -> Result<Facts, String> {
    let file = spec["file"].as_str().unwrap_or_default();
    let text = spec["text"].as_str().unwrap_or_default();
    let units = policy_of(spec["native_unit"].as_str(), spec["assume_unit"].as_str());
    let mut observed = Map::new();
    let mut produced = Vec::new();
    if file.ends_with(".csv") {
        let bound: Option<Vec<&str>> = spec["bindings"]
            .as_object()
            .map(|b| b.keys().map(String::as_str).collect());
        let read = match &bound {
            Some(b) => csv::read_bound(text, &units, file, b),
            None => csv::read(text, &units),
        };
        match read {
            Ok(params) => {
                for p in params {
                    let bound_path = spec["bindings"][&p.name].as_str();
                    observed.insert(
                        p.name,
                        Value::Object(record("value", &p.observed, bound_path)),
                    );
                }
            }
            Err(e) => {
                let code = e.to_string().chars().take(4).collect::<String>();
                tracing::warn!(error = %e, "conformance: csv read failed");
                produced.push(Produced {
                    code,
                    severity: "error".into(),
                    path: None,
                });
            }
        }
    } else if file.ends_with(".json") {
        let params = json::read(text, &units).map_err(|e| format!("json read: {e}"))?;
        for p in params {
            observed.insert(
                p.name,
                Value::Object(record("value", &p.observed, Some(&p.path))),
            );
        }
    } else {
        sw_read(text, &units, &mut observed);
    }
    let mut facts = facts_of(produced);
    facts
        .observed
        .insert("observed".into(), Value::Object(observed));
    Ok(facts)
}

/// The name quoted at the start of an equations line.
fn quoted_name(line: &str) -> Option<&str> {
    line.strip_prefix('"')?.split_once('"').map(|(n, _)| n)
}

/// The expression text of `"name" = rhs ' comment`.
fn rhs_text(line: &str) -> &str {
    let rhs = line
        .split_once('"')
        .and_then(|(_, r)| r.split_once('"'))
        .map_or("", |(_, r)| r);
    let rhs = rhs.trim_start().strip_prefix('=').unwrap_or(rhs);
    let mut quoted = false;
    for (i, c) in rhs.char_indices() {
        match c {
            '"' => quoted = !quoted,
            '\'' if !quoted => return rhs[..i].trim(),
            _ => {}
        }
    }
    rhs.trim()
}

/// Equations read: values, kept expressions and raw unparsed lines, keyed by variable name.
fn sw_read(text: &str, units: &UnitPolicy, out: &mut Map<String, Value>) {
    let lines: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('\''))
        .collect();
    for (line, read) in lines.iter().zip(sw_equations::read(text, units)) {
        let name = read
            .name
            .clone()
            .or_else(|| quoted_name(line).map(str::to_owned));
        let Some(name) = name else { continue };
        let rec = match &read.rhs {
            sw_equations::ReadRhs::Value(o) => record("value", o, None),
            sw_equations::ReadRhs::Expr(_) => {
                let mut m = Map::new();
                m.insert("kind".into(), json!("expression"));
                m.insert("text".into(), json!(rhs_text(line)));
                m.insert("exact".into(), json!(true));
                m
            }
            sw_equations::ReadRhs::Unparsed(o) => {
                let mut m = Map::new();
                m.insert("kind".into(), json!("unparsed"));
                m.insert("raw".into(), json!(o.literal));
                m.insert("exact".into(), json!(o.exact));
                m
            }
        };
        out.insert(name, Value::Object(rec));
    }
}

/// A file-backed adapter: `emit` writes one csv parameter, `read` parses it (optionally after a
/// mutation of the emitted file, to prove the round-trip contract notices drift).
struct CsvAdapter {
    dir: PathBuf,
    mutate_between: bool,
}

impl Adapter for CsvAdapter {
    fn name(&self) -> &str {
        "freecad"
    }
    fn capabilities(&self) -> Capabilities {
        Capabilities::new([Capability::Emit, Capability::Read])
    }
    fn emit(&mut self) -> AdapterResult<Outcome> {
        let file = self.dir.join("params.csv");
        fs::write(&file, "name,value,unit,note\nd,12.5,mm,\n")
            .map_err(|e| io_failure(&e.to_string()))?;
        Ok(Outcome {
            files: vec![file],
            diagnostics: vec![],
        })
    }
}

impl ReadAdapter for CsvAdapter {
    fn read(&mut self, files: &[PathBuf]) -> AdapterResult<Observed> {
        let mut out = Observed::new();
        for f in files {
            let mut text = fs::read_to_string(f).map_err(|e| io_failure(&e.to_string()))?;
            if self.mutate_between {
                text = text.replace("12.5", "99");
            }
            let units = policy_of(Some("mm"), None);
            for p in csv::read(&text, &units).map_err(|e| io_failure(&e.to_string()))? {
                out.insert(p.name, p.observed);
            }
        }
        Ok(out)
    }
}

/// `emit-read`: emit then read back through `testing::check_round_trip`; equality is observed.
pub fn emit_read(job: &Job) -> Result<Facts, String> {
    let spec = spec_of(job)?;
    let scratch = Scratch::new()?;
    let mut adapter = CsvAdapter {
        dir: scratch.0.clone(),
        mutate_between: spec["mutate_between"].as_bool().unwrap_or(false),
    };
    let expected: Observed = [(
        "d".to_owned(),
        ObservedValue {
            literal: "12.5".into(),
            unit: Some("mm".into()),
            unit_assumed: false,
            exact: true,
            reason: None,
        },
    )]
    .into();
    let equal = check_round_trip(&mut adapter, &expected).is_ok();
    tracing::info!(equal, "conformance: emit-read round trip");
    let mut facts = facts_of(Vec::new());
    facts
        .observed
        .insert("ir_values_equal".into(), json!(equal));
    Ok(facts)
}

/// An adapter failure wrapping a plain message (the test adapter has no richer error source).
fn io_failure(msg: &str) -> AdapterError {
    AdapterError::failed(AdapterDiagnostic {
        code: "IO".into(),
        severity: Severity::Error,
        message: msg.to_owned(),
    })
}
