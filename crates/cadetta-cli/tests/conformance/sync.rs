//! frob:ticket 01M4Z9WQPNGBM5J14R2CDJGJMG
//! frob:ticket 01M4ZAN1F6F1R2DFW1PHMB7P7J
//! `binding sync`: the adapter host's sync loop against a simulated native document.
//!
//! The fixture's `native` map is the document, `lock.synced` the lock and `design` the
//! freshly computed values. [`sync::run`] plans and pushes through [`Sim`], which writes the
//! document (or reports the fixture's `readback` instead); the planner is then run again on
//! the resulting state to observe the next run's class.
//!
//! The tool-session faults the host reacts to are modelled as knobs of the fixture: `regen` and
//! `restore` (R004), `document.dirty`/`read_only`/`locked` and `documents` (R009; a locked document
//! is treated as read-only, the library has no separate notion), `tool_installed`, `initialize` and
//! `adapter_platforms` (R008), and native items that are `driven`, `suppressed` or of another
//! `unit_family` (R010). `adapter.save` decides whether a clean run saves the document. Attach and
//! close behaviour is adapter-internal, so those fixtures (`running_instance`, `opened_by_adapter`,
//! `preopened`) are handed to the adapter-side harness in `session.rs`.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;

use cadetta_adapter::atomic::{AtomicOutcome, Document};
use cadetta_adapter::availability::{self, Verb as AvailVerb};
use cadetta_adapter::builtin::{csv, json as json_emit, sw_equations};
use cadetta_adapter::classify::{Class, R001};
use cadetta_adapter::rpc::{DimTolerance, FileState, LocatorStatus, PushStatus, RpcError};
use cadetta_adapter::sync::{self, Backend, Entry, Lock, Options, PushItem, ReadBack, Verb};
use cadetta_adapter::{
    Adapter, AdapterResult, Capabilities, Capability, Outcome, SyncPlan, plan_sync,
};
use serde_json::{Value, json};

use super::{Job, class_label, session};
use crate::compare::Produced;
use crate::run::Facts;

/// The default native document of a sync fixture.
const DEFAULT_FILE: &str = "cad/plate.SLDPRT";

/// The locator a design path binds to when neither the lock nor `extra_locators` names one.
const DEFAULT_LOCATOR: &str = "Plate_Thk";

/// Fixture keys outside this model; a fixture using one must not be executed as if it passed.
const UNMODELLED: &[&str] = &["attested", "drive_covers", "roles", "synced", "adapters"];

/// The simulated document: native values by locator, plus the fixture's faults.
#[derive(Clone, Default)]
struct Sim {
    native: BTreeMap<String, String>,
    tol: BTreeMap<String, DimTolerance>,
    readback: BTreeMap<String, String>,
    writes: Vec<String>,
    regen: bool,
    regen_fail: bool,
    restore_fail: bool,
    failed: bool,
    r004: Vec<String>,
    restored: bool,
}

/// The simulated document of one file: writes land in [`Sim::native`] (or the fixture's read-back).
struct SimDoc<'a> {
    sim: &'a mut Sim,
    file: String,
}

impl Document for SimDoc<'_> {
    fn read_current(&mut self, items: &[PushItem]) -> Vec<ReadBack> {
        items
            .iter()
            .map(|i| ReadBack {
                locator: i.locator.clone(),
                value: self.sim.native.get(&i.locator).cloned(),
                tolerance: self.sim.tol.get(&i.locator).cloned(),
            })
            .collect()
    }

    fn write(&mut self, items: &[PushItem]) -> Result<(), String> {
        tracing::debug!(file = %self.file, items = items.len(), "sync: simulated write");
        if self.sim.failed && self.sim.restore_fail {
            return Err("restore failed".into());
        }
        self.sim.writes.push(self.file.clone());
        for i in items {
            let got = match self.sim.readback.get(&i.locator) {
                Some(v) if !self.sim.failed => v.clone(),
                _ => i.value.clone(),
            };
            self.sim.native.insert(i.locator.clone(), got);
            match &i.tolerance {
                Some(t) => self.sim.tol.insert(i.locator.clone(), t.clone()),
                None => self.sim.tol.remove(&i.locator),
            };
        }
        Ok(())
    }

    fn rebuild(&mut self) -> Result<(), String> {
        if self.sim.regen_fail && !self.sim.failed {
            self.sim.failed = true;
            return Err("regen failed".into());
        }
        Ok(())
    }
}

impl Backend for Sim {
    fn document<'a>(&'a mut self, _adapter: &str, file: &str) -> Box<dyn Document + 'a> {
        Box::new(SimDoc {
            sim: self,
            file: file.to_owned(),
        })
    }

    fn pushed(&mut self, file: &str, regen: bool, outcome: &AtomicOutcome) {
        tracing::debug!(
            file,
            regen,
            restored = outcome.restored,
            "sync: simulated push done"
        );
        self.regen |= regen;
        self.restored |= outcome.restored;
        self.r004.extend(
            outcome
                .diagnostics
                .iter()
                .filter(|d| d.code == "R004")
                .map(|d| d.message.clone()),
        );
    }
}

/// One design value bound to its native locator.
struct Design {
    path: String,
    ours: String,
    ours_tol: Option<DimTolerance>,
    locator: String,
}

/// Static facts of a native item: its expression and whether the tool would refuse a push.
#[derive(Default)]
struct Item {
    expression: Option<String>,
    /// How the native tool names the expression: `equation` or `expression`.
    kind: &'static str,

    rejected: bool,
}

/// Split `12mm +-0.1mm` into the value and its symmetric tolerance.
fn split_tol(text: &str) -> (String, Option<DimTolerance>) {
    match text.split_once("+-") {
        Some((v, t)) => (
            v.trim().to_owned(),
            Some(DimTolerance::Symmetric {
                t: t.trim().to_owned(),
            }),
        ),
        None => (text.trim().to_owned(), None),
    }
}

/// Collapse `first push` to `push`: a locator the lock lacks is still pushed (adapters.md 2).
fn next_label(class: Class) -> &'static str {
    match class_label(class) {
        "first push" => "push",
        other => other,
    }
}

/// The document as text, one `locator = value` line each, so before/after can be diffed.
fn render(native: &BTreeMap<String, String>) -> String {
    native.iter().map(|(l, v)| format!("{l} = {v}\n")).collect()
}

/// The R008 fault the fixture describes, as the error the host would see from `initialize`.
fn availability_fault(spec: &Value) -> Option<RpcError> {
    if spec["tool_installed"].as_bool() == Some(false) {
        return Some(RpcError::Spawn(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "native tool not installed",
        )));
    }
    let expected = spec["adapter"]["version"].as_str();
    if let (Some(found), Some(expected)) = (spec["initialize"]["version"].as_str(), expected)
        && found != expected
    {
        return Some(RpcError::VersionMismatch {
            expected: expected.to_owned(),
            found: found.to_owned(),
        });
    }
    let host = spec["platform"].as_str()?;
    let platforms: Vec<String> = spec["adapter_platforms"]
        .as_array()?
        .iter()
        .filter_map(|p| p.as_str().map(str::to_owned))
        .collect();
    (!platforms.is_empty() && !platforms.iter().any(|p| p == host)).then(|| {
        RpcError::PlatformMismatch {
            host: host.to_owned(),
            platforms,
        }
    })
}

/// The files of the session with their dirty/read-only state; a locked document counts as
/// read-only (the library has no separate notion, adapters.md 4).
fn file_states(spec: &Value) -> Vec<FileState> {
    let one = |d: &Value| FileState {
        path: d["file"].as_str().unwrap_or(DEFAULT_FILE).to_owned(),
        dirty: d["dirty"].as_bool() == Some(true),
        readonly: d["read_only"].as_bool() == Some(true) || d["locked"].as_bool() == Some(true),
    };
    match spec["documents"].as_array() {
        Some(docs) => docs.iter().map(one).collect(),
        None => vec![one(&spec["document"])],
    }
}

/// What one file's sync produced.
struct FileRun {
    file: String,
    sim: Sim,
    report: sync::Report,
    rejected: Vec<String>,
    missing: Vec<String>,
    next: Option<sync::Report>,
}

/// Everything the per-file runs share.
struct Plan<'a> {
    designs: &'a [Design],
    items: &'a BTreeMap<String, Item>,
    opts: &'a Options,
}

impl Plan<'_> {
    /// The entries of `file` against the simulated native state; locators without a native
    /// item are returned separately (R003), rejected ones are left to the tool (R010).
    fn entries(
        &self,
        file: &str,
        lock: &Lock,
        sim: &Sim,
    ) -> (Vec<Entry>, Vec<&Design>, Vec<&Design>) {
        let (mut entries, mut missing, mut rejected) = (Vec::new(), Vec::new(), Vec::new());
        for d in self.designs {
            let Some(theirs) = sim.native.get(&d.locator) else {
                missing.push(d);
                continue;
            };
            let item = self.items.get(&d.locator);
            if item.is_some_and(|i| i.rejected) {
                rejected.push(d);
                continue;
            }
            entries.push(Entry {
                adapter: "fixture".into(),
                file: file.to_owned(),
                path: d.path.clone(),
                base: lock
                    .get(&("fixture".into(), d.path.clone(), d.locator.clone()))
                    .cloned(),
                replaces: item.and_then(|i| i.expression.clone()),
                locator: d.locator.clone(),
                ours: d.ours.clone(),
                theirs: theirs.clone(),
                ours_tol: d.ours_tol.clone(),
                theirs_tol: sim.tol.get(&d.locator).cloned(),
            });
        }
        (entries, missing, rejected)
    }

    /// Run the sync of one ready file against its own copy of the document.
    fn run_file(&self, file: &str, mut sim: Sim, lock: &mut Lock) -> FileRun {
        let (entries, missing, rejected) = self.entries(file, lock, &sim);
        let report = sync::run(&entries, self.opts, &mut sim, lock);
        let next = (!self.opts.dry_run).then(|| {
            let (again, _, _) = self.entries(file, lock, &sim);
            let mut scratch = lock.clone();
            sync::run(
                &again,
                &Options {
                    dry_run: true,
                    ..Options::default()
                },
                &mut sim,
                &mut scratch,
            )
        });
        tracing::debug!(file, records = report.records.len(), "sync: file run done");
        FileRun {
            file: file.to_owned(),
            sim,
            report,
            rejected: rejected.iter().map(|d| d.locator.clone()).collect(),
            missing: missing.iter().map(|d| d.locator.clone()).collect(),
            next,
        }
    }
}

/// The fixture's adapter as the planner sees it: a name and the capabilities it declares.
struct FixtureAdapter {
    name: String,
    caps: Capabilities,
}

impl FixtureAdapter {
    /// Read `adapter.id` and `adapter.capabilities`; `None` when the fixture declares none.
    fn from_spec(spec: &Value) -> Option<Self> {
        let caps: Vec<Capability> = spec["adapter"]["capabilities"]
            .as_array()?
            .iter()
            .filter_map(|c| match c.as_str()? {
                "emit" => Some(Capability::Emit),
                "read" => Some(Capability::Read),
                "drive" => Some(Capability::Drive),
                _ => None,
            })
            .collect();
        Some(Self {
            name: spec["adapter"]["id"]
                .as_str()
                .unwrap_or("fixture")
                .to_owned(),
            caps: Capabilities::new(caps),
        })
    }
}

impl Adapter for FixtureAdapter {
    fn name(&self) -> &str {
        &self.name
    }

    fn capabilities(&self) -> Capabilities {
        self.caps.clone()
    }

    fn emit(&mut self) -> AdapterResult<Outcome> {
        Ok(Outcome::default())
    }

    fn manual_step(&self) -> String {
        "relink or reload the equations file".into()
    }
}

/// The `--dry-run` plan lines of one push: the native expression it removes comes first (R006).
fn plan_lines(report: &sync::Report, items: &BTreeMap<String, Item>) -> Vec<String> {
    let mut lines = Vec::new();
    for rec in report.records.iter().filter(|r| r.verb == Verb::Push) {
        if let Some(old) = &rec.entry.replaces {
            let kind = items
                .get(&rec.entry.locator)
                .map_or("expression", |i| i.kind);
            lines.push(format!("remove {kind} {old}"));
        }
        lines.push(format!("set {}", rec.entry.ours));
    }
    lines
}

/// The emitters' header-noop check: a regenerated file differing only in its header comment
/// (or digests) is not rewritten (adapters.md 2); anything else is.
fn header_noop(spec: &Value) -> Result<Facts, String> {
    let text = |k: &str| {
        spec[k]
            .as_str()
            .ok_or_else(|| format!("header-noop fixture lacks `{k}`"))
    };
    let (existing, emitted) = (text("existing")?, text("emitted")?);
    let (path, same) = if existing.starts_with('#') {
        (
            "build/params.csv",
            csv::same_modulo_header(existing, emitted).map_err(|e| e.to_string())?,
        )
    } else if existing.starts_with('{') {
        (
            "build/params.json",
            json_emit::same_modulo_header(existing, emitted).map_err(|e| e.to_string())?,
        )
    } else {
        (
            "build/eq.txt",
            sw_equations::same_modulo_header(existing, emitted),
        )
    };
    let mut facts = Facts::default();
    facts.before.insert(path.into(), existing.to_owned());
    facts.after.insert(
        path.into(),
        if same { existing } else { emitted }.to_owned(),
    );
    tracing::info!(path, same, "sync: header-noop compared");
    Ok(facts)
}

/// Run the fixture's sync once against the simulated document.
pub fn run(job: &Job) -> Result<Facts, String> {
    let text =
        fs::read_to_string(job.fixture).map_err(|e| format!("{}: {e}", job.fixture.display()))?;
    let spec: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    if session::KEYS.iter().any(|k| spec.get(*k).is_some()) {
        return session::run(job);
    }
    if let Some(k) = UNMODELLED.iter().find(|k| spec.get(**k).is_some()) {
        return Err(format!("sync executor does not model fixture key `{k}`"));
    }
    if spec.get("existing").is_some() {
        return header_noop(&spec);
    }
    let mut facts = Facts::default();
    if let Some(err) = availability_fault(&spec) {
        let u = availability::unavailable("fixture", &err, AvailVerb::Sync)
            .ok_or("availability fault is not an R008 condition")?;
        facts.produced.push(Produced {
            code: u.diagnostic.code.clone(),
            severity: "error".into(),
            path: None,
        });
        facts.messages.push(u.diagnostic.message.clone());
        facts.exit = i32::from(u.exit.unwrap_or(1));
        facts.observed.insert("sync_ran".into(), json!(false));
        tracing::info!(exit = facts.exit, "sync: adapter unavailable, nothing ran");
        return Ok(facts);
    }
    facts.observed.insert("sync_ran".into(), json!(true));
    if let Some(adapter) = FixtureAdapter::from_spec(&spec) {
        let plan = plan_sync(&adapter).map_err(|e| e.to_string())?;
        let degraded = matches!(plan, SyncPlan::EmitWithInstructions { .. });
        facts.observed.insert("classified".into(), json!(!degraded));
        facts.observed.insert("emitted".into(), json!(degraded));
        if let SyncPlan::EmitWithInstructions { instructions } = plan {
            // No `theirs` can be read, so nothing is classified and R001 cannot arise.
            tracing::info!(adapter = %adapter.name, "sync: degraded to emit plus instructions");
            facts
                .observed
                .insert("instructions".into(), json!([instructions]));
            return Ok(facts);
        }
    }

    let args: Vec<&str> = spec["args"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    let opts = Options {
        dry_run: args.contains(&"--dry-run"),
        no_regen: args.contains(&"--no-regen"),
        force_design: args.contains(&"--force-design"),
        ..Options::default()
    };
    let mut lock = Lock::new();
    let mut by_path: BTreeMap<String, String> = BTreeMap::new();
    for s in spec["lock"]["synced"].as_array().into_iter().flatten() {
        let (p, l, v) = (
            s["path"].as_str(),
            s["locator"].as_str(),
            s["value"].as_str(),
        );
        let (Some(p), Some(l), Some(v)) = (p, l, v) else {
            return Err("lock.synced record lacks path, locator or value".into());
        };
        let (value, tol) = split_tol(v);
        lock.insert(
            ("fixture".into(), p.to_owned(), l.to_owned()),
            sync::record_literal(&value, tol.as_ref()),
        );
        by_path.insert(p.to_owned(), l.to_owned());
    }
    for (p, l) in spec["extra_locators"].as_object().into_iter().flatten() {
        by_path.insert(
            p.clone(),
            l.as_str()
                .ok_or("extra_locators value is not text")?
                .to_owned(),
        );
    }
    let extra: Vec<String> = by_path
        .iter()
        .filter(|(p, _)| spec["extra_locators"].get(p.as_str()).is_some())
        .map(|(_, l)| l.clone())
        .collect();
    let mut sim = Sim {
        readback: spec["readback"]
            .as_object()
            .into_iter()
            .flatten()
            .filter_map(|(k, v)| v.as_str().map(|v| (k.clone(), v.to_owned())))
            .collect(),
        regen_fail: spec["regen"].as_str().unwrap_or("ok") != "ok",
        restore_fail: spec["restore"].as_str().unwrap_or("ok") != "ok",
        ..Sim::default()
    };
    let mut items = BTreeMap::new();
    for (l, v) in spec["native"]
        .as_object()
        .ok_or("sync fixture lacks `native`")?
    {
        sim.native.insert(
            l.clone(),
            v["value"]
                .as_str()
                .ok_or("native item lacks `value`")?
                .to_owned(),
        );
        if let Some(t) = v["tolerance"].as_str() {
            sim.tol.insert(
                l.clone(),
                DimTolerance::Symmetric {
                    t: t.strip_prefix("+-")
                        .ok_or("native tolerance is not `+-t`")?
                        .trim()
                        .to_owned(),
                },
            );
        }
        items.insert(
            l.clone(),
            Item {
                expression: v["expression"]
                    .as_str()
                    .or_else(|| v["equation"].as_str())
                    .map(str::to_owned),
                kind: if v["equation"].is_string() {
                    "equation"
                } else {
                    "expression"
                },
                rejected: v["driven"].as_bool() == Some(true)
                    || v["suppressed"].as_bool() == Some(true)
                    || v["unit_family"].as_str().is_some_and(|u| u != "Length"),
            },
        );
    }
    let designs: Vec<Design> = spec["design"]
        .as_object()
        .ok_or("sync fixture lacks `design`")?
        .iter()
        .map(|(p, v)| {
            let (ours, ours_tol) = split_tol(v.as_str().unwrap_or_default());
            Design {
                locator: by_path
                    .get(p)
                    .cloned()
                    .unwrap_or_else(|| DEFAULT_LOCATOR.to_owned()),
                path: p.clone(),
                ours,
                ours_tol,
            }
        })
        .collect();
    let plan = Plan {
        designs: &designs,
        items: &items,
        opts: &opts,
    };

    // R009: a dirty or read-only document is refused before anything is pushed into it.
    let (ready, busy) = availability::partition_busy(&file_states(&spec));
    let before = lock.clone();
    let native0 = sim.native.clone();
    let mut runs = Vec::new();
    let mut merged = lock.clone();
    for file in &ready {
        let mut file_lock = lock.clone();
        let run = plan.run_file(file, sim.clone(), &mut file_lock);
        merged.extend(file_lock);
        runs.push(run);
    }
    let lock = merged;

    let mut r001_exit = false;
    let mut error_exit = !busy.is_empty();
    for b in &busy {
        facts.produced.push(Produced {
            code: b.diagnostic.code.clone(),
            severity: "error".into(),
            path: None,
        });
        facts.messages.push(b.diagnostic.message.clone());
    }
    for r in &runs {
        for d in r
            .report
            .diagnostics
            .iter()
            .filter(|d| !(opts.force_design && d.code == R001))
        {
            let severity = if d.code == sync::R006 {
                "warning"
            } else {
                "error"
            };
            r001_exit |= d.code == R001;
            facts.produced.push(Produced {
                code: d.code.to_owned(),
                severity: severity.into(),
                path: None,
            });
            facts.messages.push(d.message.clone());
        }
        // R003: a recorded native item is gone; its locator is skipped, the rest proceeds.
        let pairs: Vec<(String, String)> = designs
            .iter()
            .map(|d| (d.path.clone(), d.locator.clone()))
            .collect();
        let found: BTreeSet<(String, String)> = designs
            .iter()
            .filter(|d| !r.missing.contains(&d.locator))
            .map(|d| (d.path.clone(), d.locator.clone()))
            .collect();
        for d in sync::missing_implicit("fixture", &pairs, &found, &before) {
            error_exit = true;
            facts.produced.push(Produced {
                code: d.code.to_owned(),
                severity: "error".into(),
                path: d.message.split_once(": ").map(|(p, _)| p.to_owned()),
            });
            facts.messages.push(d.message);
        }
        if !r.sim.r004.is_empty() {
            // One R004 per file; the restore failure rides along in the messages.
            error_exit = true;
            facts.produced.push(Produced {
                code: "R004".into(),
                severity: "error".into(),
                path: None,
            });
            facts.messages.extend(r.sim.r004.iter().cloned());
        }
        // R010: the tool refused these locators; one diagnostic per file.
        let statuses: Vec<LocatorStatus> = r
            .rejected
            .iter()
            .map(|l| LocatorStatus {
                locator: l.clone(),
                status: PushStatus::Rejected,
                old: None,
                new: None,
                tolerance: None,
            })
            .collect();
        for rej in availability::rejections(&[(r.file.clone(), statuses)]) {
            error_exit = true;
            let path = designs
                .iter()
                .find(|d| rej.locators.contains(&d.locator))
                .map(|d| d.path.clone());
            facts.produced.push(Produced {
                code: rej.diagnostic.code.clone(),
                severity: "error".into(),
                path,
            });
            facts.messages.push(rej.diagnostic.message.clone());
        }
    }
    facts.exit = i32::from((r001_exit && !opts.force_design) || error_exit);
    if opts.dry_run {
        let lines: Vec<String> = runs
            .iter()
            .flat_map(|r| plan_lines(&r.report, &items))
            .collect();
        facts.stdout = lines.join("\n");
        facts.observed.insert("dry_run_shows".into(), json!(lines));
    }

    let obs = &mut facts.observed;
    obs.insert(
        "class".into(),
        json!(
            runs.iter()
                .flat_map(|r| r.report.records.iter())
                .map(|r| class_label(r.class))
                .collect::<Vec<_>>()
        ),
    );
    let touched: Vec<&FileRun> = runs.iter().filter(|r| !r.sim.writes.is_empty()).collect();
    for r in &runs {
        facts.before.insert(r.file.clone(), render(&native0));
        if !r.sim.writes.is_empty() {
            facts.after.insert(r.file.clone(), render(&r.sim.native));
        }
    }
    let obs = &mut facts.observed;
    obs.insert("documents_touched".into(), json!(!touched.is_empty()));
    obs.insert("regen".into(), json!(runs.iter().any(|r| r.sim.regen)));
    obs.insert(
        "lock_updated".into(),
        json!(runs.iter().any(|r| r.report.lock_written)),
    );
    // Records that already existed in the lock come first, newly adopted ones after.
    let changed = |existing: bool| {
        lock.iter()
            .filter(|(k, v)| before.get(*k) != Some(*v) && before.contains_key(*k) == existing)
            .map(|(k, _)| k.2.as_str())
            .collect::<Vec<_>>()
    };
    let mut updated = changed(true);
    updated.extend(changed(false));
    obs.insert("lock_synced_updated".into(), json!(updated));

    let mut status = serde_json::Map::new();
    for r in &runs {
        for rec in &r.report.records {
            let entry = match rec.verb {
                Verb::Push => {
                    json!({"status": "pushed", "old": rec.entry.theirs, "new": rec.entry.ours})
                }
                Verb::Fail => {
                    json!({"status": "failed", "old": rec.entry.theirs, "new": rec.entry.ours})
                }
                _ => continue,
            };
            status.insert(rec.entry.locator.clone(), entry);
        }
        for l in &r.rejected {
            status.insert(l.clone(), json!("rejected"));
        }
        for l in &r.missing {
            status.insert(l.clone(), json!("skipped"));
        }
    }
    obs.insert("push_status".into(), Value::Object(status));
    obs.insert(
        "pushes".into(),
        json!(
            runs.iter()
                .flat_map(|r| r.report.records.iter())
                .filter(|r| r.verb == Verb::Push)
                .map(|r| r.entry.locator.as_str())
                .collect::<Vec<_>>()
        ),
    );
    obs.insert(
        "refused".into(),
        json!(busy.iter().map(|b| b.file.as_str()).collect::<Vec<_>>()),
    );
    obs.insert(
        "pushed_documents".into(),
        json!(touched.iter().map(|r| r.file.as_str()).collect::<Vec<_>>()),
    );
    obs.insert("session_closed".into(), json!(true));
    let native_after: serde_json::Map<String, Value> = runs
        .first()
        .map_or(&native0, |r| &r.sim.native)
        .iter()
        .map(|(l, v)| (l.clone(), json!(v)))
        .collect();
    obs.insert("native_after".into(), Value::Object(native_after));
    let any_r004 = runs.iter().any(|r| !r.sim.r004.is_empty());
    obs.insert(
        "restored".into(),
        json!(runs.iter().any(|r| r.sim.restored)),
    );
    obs.insert(
        "rebuild_errors_reported_as_diagnostics".into(),
        json!(any_r004),
    );

    // Save (adapters.md 2): after a clean push, unless the adapter never saves.
    let saves = spec["adapter"]["save"].as_str().unwrap_or("after-sync") == "after-sync";
    let blocked = |r: &FileRun| {
        !r.sim.r004.is_empty()
            || !r.rejected.is_empty()
            || r.report.records.iter().any(|x| x.verb == Verb::Fail)
    };
    let saved: Vec<&str> = touched
        .iter()
        .filter(|r| {
            saves && !opts.dry_run && !opts.no_regen && !blocked(r) && r.report.lock_written
        })
        .map(|r| r.file.as_str())
        .collect();
    obs.insert("saved".into(), json!(saved));
    obs.insert("lock_records_file_digest".into(), json!(!saved.is_empty()));
    if !touched.is_empty() && saved.is_empty() && !touched.iter().any(|r| blocked(r)) {
        obs.insert("state".into(), json!("open"));
        obs.insert("reason".into(), json!("unsaved"));
    }

    let adopted: Vec<&str> = spec["lock"]["adopted"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|a| a["file"].as_str())
        .collect();
    if !adopted.is_empty() {
        let done = |f: &str| {
            !opts.dry_run
                && runs.iter().any(|r| {
                    r.file == f
                        && !blocked(r)
                        && !r.report.records.iter().any(|x| x.verb == Verb::Fail)
                })
        };
        obs.insert("adopted_before".into(), json!(adopted));
        obs.insert(
            "adopted_after".into(),
            json!(adopted.iter().filter(|f| !done(f)).collect::<Vec<_>>()),
        );
    }

    if !opts.dry_run {
        let again: Vec<&sync::Report> = runs.iter().filter_map(|r| r.next.as_ref()).collect();
        let classes: serde_json::Map<String, Value> = again
            .iter()
            .flat_map(|a| a.records.iter())
            .filter(|r| extra.contains(&r.entry.locator))
            .map(|r| (r.entry.locator.clone(), json!(next_label(r.class))))
            .collect();
        let all: Vec<_> = again.iter().flat_map(|a| a.records.iter()).collect();
        if let [only] = all.as_slice() {
            obs.insert("next_run_class".into(), json!(next_label(only.class)));
        }
        obs.insert("next_run_classes".into(), Value::Object(classes));
    }
    tracing::info!(
        files = runs.len(),
        refused = busy.len(),
        exit = facts.exit,
        "sync: fixture executed"
    );
    Ok(facts)
}
