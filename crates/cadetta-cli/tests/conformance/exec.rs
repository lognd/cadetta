//! frob:ticket 01M4V1X55R20C1PT8B56BCVN48
//! The entry dispatcher: one executor per (kind, verb), each producing [`Facts`].

use std::fs;
use std::path::Path;

use cadetta_adapter::sync::{self, Backend, Entry, Lock, Options, PushItem, ReadBack};
use cadetta_units::{Magnitude, Rational, UnitTable};
use serde_json::{Value, json};

use crate::binding;
use crate::compare::Produced;
use crate::expect::{Block, Expect};
#[path = "import.rs"]
mod import_exec; // nested like `protocol`, for the same reason
#[path = "protocol.rs"]
mod protocol; // nested so every conformance target gets it without a new `mod` line
#[path = "push_plan.rs"]
mod push_plan; // nested like `protocol`, for the same reason
#[path = "read.rs"]
mod read; // nested like `protocol`, for the same reason
#[path = "sync.rs"]
mod sync_exec; // nested like `protocol`, for the same reason
use crate::run::{Facts, Scratch, parse_json, run_cli, stage};

/// What an executor is asked to run.
pub struct Job<'a> {
    /// The fixture file or package directory.
    pub fixture: &'a Path,
    /// The `entry` split on whitespace; the first token is the verb.
    pub entry: Vec<String>,
    /// The block being checked (`args` is read from here).
    pub block: &'a Block,
}

/// One executor.
type Exec = fn(&Job) -> Result<Facts, String>;

/// Dispatch table: (`kind`, entry verb) to executor. A pair absent here has no
/// executor and the fixture fails with a message saying so; nothing is skipped.
const EXECUTORS: &[(&str, &str, Exec)] = &[
    ("source", "check", cli),
    ("source", "config", cli),
    ("source", "ack", cli),
    ("source", "emit", cli),
    ("source", "solve", cli),
    ("source", "fmt", cli),
    ("source", "names", cli),
    ("source", "bind", cli),
    ("source", "cite", cli),
    ("source", "eval", eval),
    ("source", "init", init),
    ("binding", "classify", classify),
    ("binding", "emit", binding::emit_scope),
    ("binding", "import", import_exec::run),
    ("binding", "protocol", protocol::run),
    ("binding", "push-plan", push_plan::run),
    ("binding", "read", read::read),
    ("binding", "sync", sync_exec::run),
    ("binding", "emit-read", read::emit_read),
    ("binding", "emit-twice", binding::emit_twice),
    ("binding", "emit-compare", binding::emit_compare),
    ("binding", "impact-swap", binding::impact_swap),
];

/// Run the fixture with the executor registered for the expectation's kind and entry.
pub fn execute(expect: &Expect, fixture: &Path, block: &Block) -> Result<Facts, String> {
    let entry: Vec<String> = expect
        .entry
        .as_deref()
        .unwrap_or("check")
        .split_whitespace()
        .map(str::to_owned)
        .collect();
    let verb = entry.first().map_or("check", String::as_str).to_owned();
    let job = Job {
        fixture,
        entry,
        block,
    };
    match EXECUTORS
        .iter()
        .find(|(k, v, _)| *k == expect.kind && *v == verb)
    {
        Some((_, _, exec)) => exec(&job),
        None => Err(format!(
            "no executor for kind `{}` entry `{verb}`",
            expect.kind
        )),
    }
}

/// The CLI arguments of a job: the entry tokens (`config` is `check`) plus the block's `args`.
fn cli_args(job: &Job) -> Vec<String> {
    let mut args = job.entry.clone();
    if args.first().is_some_and(|v| v == "config") {
        args[0] = "check".into();
    }
    if let Some(extra) = job.block.extra.get("args").and_then(Value::as_array) {
        args.extend(extra.iter().filter_map(|a| a.as_str().map(str::to_owned)));
    }
    args
}

/// Stage the fixture as a package and run the entry as a CLI invocation.
fn cli(job: &Job) -> Result<Facts, String> {
    let scratch = stage(job.fixture)?;
    let mut args = cli_args(job);
    resolve_adapter_token(&scratch.0, &mut args);
    run_cli(&scratch.0, &args)
}

/// `emit TOKEN` names an adapter; the CLI wants its id, so map name to id via `cadetta.toml`.
fn resolve_adapter_token(root: &Path, args: &mut [String]) {
    if args.first().map(String::as_str) != Some("emit") || args.len() < 2 {
        return;
    }
    let Ok(text) = fs::read_to_string(root.join("cadetta.toml")) else {
        return;
    };
    let Ok(doc) = text.parse::<toml::Table>() else {
        return;
    };
    let Some(list) = doc.get("adapter").and_then(toml::Value::as_array) else {
        return;
    };
    for entry in list.iter().filter_map(toml::Value::as_table) {
        if entry.get("name").and_then(toml::Value::as_str) == Some(args[1].as_str())
            && let Some(id) = entry.get("id").and_then(toml::Value::as_str)
        {
            args[1] = id.to_owned();
            return;
        }
    }
}

/// `n/d` or `n/d*pi^k` for an exact magnitude.
fn magnitude_text(m: &Magnitude) -> String {
    let r = format!("{}/{}", m.rational().numer(), m.rational().denom());
    match m.pi_exp() {
        0 => r,
        k => format!("{r}*pi^{k}"),
    }
}

/// Convert a quantity literal such as `5mm` or `-0.5` to its exact SI magnitude.
fn literal_magnitude(literal: &str) -> Result<Magnitude, String> {
    let split = literal
        .find(|c: char| !(c.is_ascii_digit() || matches!(c, '.' | '-' | '+' | 'e' | 'E' | '/')))
        .unwrap_or(literal.len());
    let (num, unit) = literal.split_at(split);
    let value = match num.split_once('/') {
        Some((n, d)) => {
            let n = Rational::parse_decimal(n).map_err(|e| format!("literal `{literal}`: {e}"))?;
            let d = Rational::parse_decimal(d).map_err(|e| format!("literal `{literal}`: {e}"))?;
            n.div(&d).map_err(|e| format!("literal `{literal}`: {e}"))?
        }
        None => Rational::parse_decimal(num).map_err(|e| format!("literal `{literal}`: {e}"))?,
    };
    let factor = if unit.is_empty() {
        Magnitude::new(Rational::one(), 0)
    } else {
        UnitTable::builtin()
            .parse(unit)
            .map_err(|e| format!("literal `{literal}`: {e}"))?
            .factor
    };
    Ok(Magnitude::new(value, 0).mul(&factor))
}

/// Expected values in `n/d[*pi^k]` form, keyed by member path.
pub fn expected_members(block: &Block) -> Result<Vec<(String, String)>, String> {
    block
        .values
        .iter()
        .map(|v| {
            let path = v["path"].as_str().ok_or("value lacks `path`")?;
            let nominal = v["nominal"].as_str().ok_or("value lacks `nominal`")?;
            if nominal.starts_with('"') {
                return Ok((path.to_owned(), nominal.to_owned()));
            }
            Ok((
                path.to_owned(),
                magnitude_text(&literal_magnitude(nominal)?),
            ))
        })
        .collect()
}

/// `eval`: run `cadetta ir` and collect each member's exact nominal, plus the
/// info diagnostics (such as Q006) that `cadetta check` reports for the same package.
fn eval(job: &Job) -> Result<Facts, String> {
    let scratch = stage(job.fixture)?;
    let mut facts = run_cli(&scratch.0, &["ir".to_owned()])?;
    let checked = run_cli(&scratch.0, &["check".to_owned()])?;
    facts.produced.extend(
        checked
            .produced
            .into_iter()
            .filter(|p| p.severity == "info"),
    );
    let Some(ir) = parse_json(&facts.stdout) else {
        return Ok(facts);
    };
    for comp in ir["components"].as_array().into_iter().flatten() {
        for member in comp["members"].as_array().into_iter().flatten() {
            let (Some(path), Some(r)) = (
                member["path"].as_str(),
                member["value"]["nominal"]["r"].as_str(),
            ) else {
                continue;
            };
            let text = match member["value"]["nominal"]["pi"].as_i64().unwrap_or(0) {
                0 => r.to_owned(),
                k => format!("{r}*pi^{k}"),
            };
            facts.members.insert(path.to_owned(), text);
        }
    }
    Ok(facts)
}

/// `init`: run the fixture's `args` in an empty directory, then test the written
/// `cadetta.toml` for `block_contains`; the exit is 1 when the block is absent.
fn init(job: &Job) -> Result<Facts, String> {
    let text =
        fs::read_to_string(job.fixture).map_err(|e| format!("{}: {e}", job.fixture.display()))?;
    let spec: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let args: Vec<String> = spec["args"]
        .as_array()
        .ok_or("init fixture lacks `args`")?
        .iter()
        .filter_map(|a| a.as_str().map(str::to_owned))
        .collect();
    let scratch = Scratch::new()?;
    let mut facts = run_cli(&scratch.0, &args)?;
    if let Some(block) = spec["block_contains"].as_str()
        && facts.exit == 0
        && !facts
            .after
            .get("cadetta.toml")
            .is_some_and(|t| t.contains(block))
    {
        facts.exit = 1;
    }
    Ok(facts)
}

/// A backend that must never be reached: `classify` only plans (dry run).
struct NoBackend;

impl Backend for NoBackend {
    fn push(
        &mut self,
        _adapter: &str,
        _file: &str,
        _items: &[PushItem],
        _regen: bool,
    ) -> Vec<ReadBack> {
        unreachable!("classify fixtures run as a dry run")
    }
}

/// `classify`: one locator through the adapter host's three-way sync planner.
fn classify(job: &Job) -> Result<Facts, String> {
    let text =
        fs::read_to_string(job.fixture).map_err(|e| format!("{}: {e}", job.fixture.display()))?;
    let spec: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let field = |k: &str| spec[k].as_str().map(str::to_owned);
    let entry = Entry {
        adapter: "fixture".into(),
        file: "fixture.doc".into(),
        path: field("path").ok_or("classify fixture lacks `path`")?,
        locator: field("locator").ok_or("classify fixture lacks `locator`")?,
        base: field("base"),
        ours: field("ours").ok_or("classify fixture lacks `ours`")?,
        theirs: field("theirs").ok_or("classify fixture lacks `theirs`")?,
        replaces: field("native_expression"),
        ours_tol: None,
        theirs_tol: None,
    };
    let opts = Options {
        dry_run: true,
        force_design: spec["force_design"].as_bool().unwrap_or(false),
        ..Options::default()
    };
    let report = sync::run(&[entry], &opts, &mut NoBackend, &mut Lock::new());
    let record = report.records.first().ok_or("sync produced no record")?;
    let mut facts = Facts {
        exit: i32::from(report.has_r001()),
        ..Facts::default()
    };
    for d in &report.diagnostics {
        let severity = if d.code == sync::R006 {
            "warning"
        } else {
            "error"
        };
        facts.produced.push(Produced {
            code: d.code.to_owned(),
            severity: severity.to_owned(),
            path: None,
        });
        facts.messages.push(d.message.clone());
    }
    facts
        .observed
        .insert("class".into(), json!(class_label(record.class)));
    facts
        .observed
        .insert("action".into(), json!(action_label(record.verb)));
    Ok(facts)
}

/// The corpus spelling of a sync class (adapters.md 2 table).
pub(super) fn class_label(class: cadetta_adapter::classify::Class) -> &'static str {
    use cadetta_adapter::classify::Class;
    match class {
        Class::NoOp => "no-op",
        Class::Push => "push",
        Class::CadMoved => "cad-moved",
        Class::Converged => "converged",
        Class::BothMoved => "both-moved",
        Class::Adopt => "adopt",
        Class::FirstPush => "first push",
    }
}

/// The corpus spelling of the action a verb records.
fn action_label(verb: sync::Verb) -> &'static str {
    use sync::Verb;
    match verb {
        Verb::Push => "push ours",
        Verb::Skip | Verb::Drift => "none",
        Verb::Converged => "record only",
        Verb::Fail => "push failed",
    }
}
