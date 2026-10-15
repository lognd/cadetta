//! frob:ticket 01M4Z9WQPNGBM5J14R2CDJGJMG
//! `binding sync`: the adapter host's sync loop against a simulated native document.
//!
//! The fixture's `native` map is the document, `lock.synced` the lock and `design` the
//! freshly computed values. [`sync::run`] plans and pushes through [`Sim`], which writes the
//! document (or reports the fixture's `readback` instead); the planner is then run again on
//! the resulting state to observe the next run's class. Fixtures that need a tool session
//! (R004, R008-R010, save, attach) are outside this model and fail loudly instead of passing.

use std::collections::BTreeMap;
use std::fs;

use cadetta_adapter::atomic::{AtomicOutcome, Document};
use cadetta_adapter::classify::{Class, R001};
use cadetta_adapter::sync::{self, Backend, Entry, Lock, Options, PushItem, ReadBack, Verb};
use serde_json::{Value, json};

use super::{Job, class_label};
use crate::compare::Produced;
use crate::run::Facts;

/// The default native document of a sync fixture.
const DEFAULT_FILE: &str = "cad/plate.SLDPRT";

/// The locator a design path binds to when neither the lock nor `extra_locators` names one.
const DEFAULT_LOCATOR: &str = "Plate_Thk";

/// Fixture keys outside this model; a fixture using one must not be executed as if it passed.
const UNMODELLED: &[&str] = &[
    "attested",
    "drive_covers",
    "emitted",
    "existing",
    "initialize",
    "opened_by_adapter",
    "platform",
    "adapter_platforms",
    "adapters",
    "documents",
    "preopened",
    "roles",
    "running_instance",
    "synced",
    "tool_installed",
    "locator_kind",
];

/// The simulated document: native values by locator, plus read-backs that override a push.
struct Sim {
    native: BTreeMap<String, String>,
    readback: BTreeMap<String, String>,
    writes: Vec<String>,
    regen: bool,
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
                tolerance: None,
            })
            .collect()
    }

    fn write(&mut self, items: &[PushItem]) -> Result<(), String> {
        tracing::debug!(file = %self.file, items = items.len(), "sync: simulated write");
        self.sim.writes.push(self.file.clone());
        for i in items {
            let got = self
                .sim
                .readback
                .get(&i.locator)
                .cloned()
                .unwrap_or_else(|| i.value.clone());
            self.sim.native.insert(i.locator.clone(), got);
        }
        Ok(())
    }

    fn rebuild(&mut self) -> Result<(), String> {
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

    fn pushed(&mut self, file: &str, regen: bool, _outcome: &AtomicOutcome) {
        tracing::debug!(file, regen, "sync: simulated push done");
        self.regen |= regen;
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

/// Run the fixture's sync once against the simulated document.
pub fn run(job: &Job) -> Result<Facts, String> {
    let text =
        fs::read_to_string(job.fixture).map_err(|e| format!("{}: {e}", job.fixture.display()))?;
    let spec: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    if let Some(k) = UNMODELLED.iter().find(|k| spec.get(**k).is_some()) {
        return Err(format!("sync executor does not model fixture key `{k}`"));
    }
    let doc = &spec["document"];
    if spec["regen"].as_str().unwrap_or("ok") != "ok"
        || spec["restore"].as_str().unwrap_or("ok") != "ok"
        || ["dirty", "read_only", "locked"]
            .iter()
            .any(|f| doc[*f].as_bool() == Some(true))
    {
        return Err("sync executor does not model tool-session faults".into());
    }
    let file = doc["file"].as_str().unwrap_or(DEFAULT_FILE).to_owned();
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
        lock.insert(("fixture".into(), p.to_owned(), l.to_owned()), v.to_owned());
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
    let mut native = BTreeMap::new();
    let mut expressions = BTreeMap::new();
    for (l, v) in spec["native"]
        .as_object()
        .ok_or("sync fixture lacks `native`")?
    {
        native.insert(
            l.clone(),
            v["value"]
                .as_str()
                .ok_or("native item lacks `value`")?
                .to_owned(),
        );
        if let Some(e) = v["expression"].as_str() {
            expressions.insert(l.clone(), e.to_owned());
        }
    }
    let design: Vec<(String, String)> = spec["design"]
        .as_object()
        .ok_or("sync fixture lacks `design`")?
        .iter()
        .map(|(p, v)| (p.clone(), v.as_str().unwrap_or_default().to_owned()))
        .collect();
    let mut sim = Sim {
        native,
        readback: spec["readback"]
            .as_object()
            .into_iter()
            .flatten()
            .filter_map(|(k, v)| v.as_str().map(|v| (k.clone(), v.to_owned())))
            .collect(),
        writes: Vec::new(),
        regen: false,
    };
    let entries = |lock: &Lock, sim: &Sim| -> Result<Vec<Entry>, String> {
        design
            .iter()
            .map(|(p, ours)| {
                let locator = by_path
                    .get(p)
                    .cloned()
                    .unwrap_or_else(|| DEFAULT_LOCATOR.to_owned());
                let theirs = sim
                    .native
                    .get(&locator)
                    .ok_or_else(|| format!("no native item `{locator}`"))?
                    .clone();
                Ok(Entry {
                    adapter: "fixture".into(),
                    file: file.clone(),
                    path: p.clone(),
                    base: lock
                        .get(&("fixture".into(), p.clone(), locator.clone()))
                        .cloned(),
                    replaces: expressions.get(&locator).cloned(),
                    locator,
                    ours: ours.clone(),
                    theirs,
                    ours_tol: None,
                    theirs_tol: None,
                })
            })
            .collect()
    };
    let before = lock.clone();
    let native0 = sim.native.clone();
    let planned = entries(&lock, &sim)?;
    let report = sync::run(&planned, &opts, &mut sim, &mut lock);
    let mut facts = Facts {
        exit: i32::from(report.has_r001()),
        ..Facts::default()
    };
    for d in report
        .diagnostics
        .iter()
        .filter(|d| !(opts.force_design && d.code == R001))
    {
        let severity = if d.code == sync::R006 {
            "warning"
        } else {
            "error"
        };
        facts.produced.push(Produced {
            code: d.code.to_owned(),
            severity: severity.into(),
            path: None,
        });
        facts.messages.push(d.message.clone());
    }
    let obs = &mut facts.observed;
    obs.insert(
        "class".into(),
        json!(
            report
                .records
                .iter()
                .map(|r| class_label(r.class))
                .collect::<Vec<_>>()
        ),
    );
    sim.writes.dedup();
    facts.before.insert(file.clone(), render(&native0));
    if !sim.writes.is_empty() {
        facts.after.insert(file.clone(), render(&sim.native));
    }
    let obs = &mut facts.observed;
    obs.insert("documents_touched".into(), json!(!sim.writes.is_empty()));
    obs.insert("regen".into(), json!(sim.regen));
    obs.insert("lock_updated".into(), json!(report.lock_written));
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
    let status: serde_json::Map<String, Value> = report
        .records
        .iter()
        .filter(|r| matches!(r.verb, Verb::Push | Verb::Fail))
        .map(|r| {
            let s = if r.verb == Verb::Push {
                "pushed"
            } else {
                "failed"
            };
            (
                r.entry.locator.clone(),
                json!({"status": s, "old": r.entry.theirs, "new": r.entry.ours}),
            )
        })
        .collect();
    obs.insert("push_status".into(), Value::Object(status));
    if !opts.dry_run {
        let again = sync::run(
            &entries(&lock, &sim)?,
            &Options {
                dry_run: true,
                ..Options::default()
            },
            &mut sim,
            &mut lock,
        );
        let classes: serde_json::Map<String, Value> = again
            .records
            .iter()
            .filter(|r| extra.contains(&r.entry.locator))
            .map(|r| (r.entry.locator.clone(), json!(next_label(r.class))))
            .collect();
        if let [only] = again.records.as_slice() {
            obs.insert("next_run_class".into(), json!(next_label(only.class)));
        }
        obs.insert("next_run_classes".into(), Value::Object(classes));
    }
    tracing::info!(
        records = report.records.len(),
        exit = facts.exit,
        "sync: fixture executed"
    );
    Ok(facts)
}
