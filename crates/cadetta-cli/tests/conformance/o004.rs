//! frob:ticket 01M4VQXMBWSFHWEVGFBBC7S5KT
//! `binding sync` O004 fixtures (adapters.md 2, obligations.md 2.3): the "CAD file reflects
//! design" obligation of a native file an adapter without `drive` handed over.
//!
//! This is observed on the real CLI, not on a model: the fixture is staged as a package with
//! the named built-in adapters, `cadetta sync` runs, and the obligation is open exactly when
//! the run reported O004. `attested` runs `cadetta ack sync:FILE` and syncs again (an `attest`
//! lock record on `sync:FILE` discharges it). `drive_covers` stages a drive adapter, stubbed
//! by a script that never runs, with a `synced file=` record for that file; such a file is
//! judged by the drive adapter, so cadetta must not report it open.
//!
//! Two fixture shapes select this executor (see [`applies`]): the emitter shape
//! (`adapters`, `synced`, optional `attested` and `drive_covers`) and the adapter model whose
//! block expects an `obligation` from an adapter that lacks `drive`. Observed: `obligation`,
//! `state` (`open` or `current`) and `reason`.

use std::path::Path;

use serde_json::Value;

use super::Job;
use super::protocol::{put, put_exec};
use crate::run::{Facts, Scratch, run_cli, writing};

/// The obligation every O004 fixture names (adapters.md 2).
pub const OBLIGATION: &str = "CAD file reflects design";

/// The role `cadetta ack sync:FILE` attests under when the fixture names none.
const DEFAULT_ROLE: &str = "cad_review";

/// The native file of an emitter-shape fixture that names none.
const DEFAULT_FILE: &str = "cad/plate.SLDPRT";

/// Built-in adapters that cannot drive a tool.
const BUILTIN: &[&str] = &["csv", "json", "sw-equations"];

/// A stand-in digest for the record of a drive adapter's file; nothing reads it.
const DRIVE_DIGEST: &str = "0000000000000000000000000000000000000000000000000000000000000000";

/// Whether `spec` and `job` are an O004 fixture this executor runs.
pub fn applies(spec: &Value, job: &Job) -> bool {
    if spec.get("adapters").is_some() || spec.get("attested").is_some() {
        return true;
    }
    let caps = spec["adapter"]["capabilities"].as_array();
    let drives = caps.is_some_and(|c| c.iter().any(|v| v == "drive"));
    job.block.extra.contains_key("obligation") && !drives
}

/// The adapter ids to stage, in fixture order.
fn adapters(spec: &Value) -> Vec<String> {
    match spec["adapters"].as_array() {
        Some(list) => list
            .iter()
            .filter_map(|v| v.as_str().map(str::to_owned))
            .collect(),
        None => spec["adapter"]["id"]
            .as_str()
            .map(str::to_owned)
            .into_iter()
            .collect(),
    }
}

/// The design members as `(model, member, value)`; the emitter shape has one default member.
fn design(spec: &Value) -> Vec<(String, String, String)> {
    let Some(map) = spec["design"].as_object() else {
        return vec![("CoverPlate".into(), "thickness".into(), "6mm".into())];
    };
    map.iter()
        .filter_map(|(path, v)| {
            let (_, tail) = path.rsplit_once("::")?;
            let (model, member) = tail.split_once('.')?;
            Some((model.to_owned(), member.to_owned(), v.as_str()?.to_owned()))
        })
        .collect()
}

/// The package name of the fixture's design paths (`acme.demo` unless the paths say otherwise).
fn package_of(spec: &Value) -> String {
    spec["design"]
        .as_object()
        .and_then(|m| m.keys().next())
        .and_then(|p| p.split_once("::"))
        .map_or_else(|| "acme.plate".to_owned(), |(pkg, _)| pkg.to_owned())
}

/// Write the scratch package for the fixture; returns the native file the adapters hand over.
fn build(spec: &Value, root: &Path) -> Result<String, String> {
    let file = spec["synced"]
        .as_str()
        .or_else(|| spec["document"]["file"].as_str())
        .unwrap_or(DEFAULT_FILE)
        .to_owned();
    let members = design(spec);
    let (pkg, model) = (package_of(spec), members[0].0.clone());
    let fields: String = members
        .iter()
        .map(|(_, m, v)| format!("  {m}: Length = {v}\n"))
        .collect();
    let role = spec["roles"]
        .as_object()
        .and_then(|r| r.keys().next().cloned())
        .unwrap_or_else(|| DEFAULT_ROLE.to_owned());
    let mut toml = format!("[package]\nname = \"{pkg}\"\nversion = \"0.1.0\"\n");
    let covers = spec["drive_covers"].as_str();
    for id in adapters(spec) {
        if BUILTIN.contains(&id.as_str()) {
            toml.push_str(&format!(
                "[[adapter]]\nname = \"{id}\"\nfiles = [\"{file}\"]\n"
            ));
            let bound = members
                .iter()
                .map(|(m, member, _)| {
                    format!(
                        "[\"{pkg}::{m}.{member}\"]\nlocators = [ {{ locator = \"Plate_Thk\", kind = \"global\", native_unit = \"mm\" }} ]\n"
                    )
                })
                .collect::<String>();
            put(root, &format!("bindings/{id}.toml"), &bound)?;
        } else if covers.is_some() {
            toml.push_str(&format!(
                "[[adapter]]\nname = \"{id}\"\ncommand = [\"drive.sh\"]\nfiles = [\"{file}\"]\n"
            ));
            put_exec(root, "drive.sh", "#!/bin/sh\nexit 1\n")?;
        }
    }
    toml.push_str(&format!(
        "[obligations.roles.{role}]\ncovers = [\"sync:**\"]\n"
    ));
    put(root, "cadetta.toml", &toml)?;
    put(
        root,
        "src/lib.cdt",
        &format!("model {model} {{\n{fields}}}\n"),
    )?;
    if let Some(covered) = covers {
        let driver = adapters(spec)
            .into_iter()
            .find(|a| !BUILTIN.contains(&a.as_str()))
            .ok_or("drive_covers without a drive adapter")?;
        let line = format!("synced {driver} file={covered} digest={DRIVE_DIGEST}\n");
        put(root, "cadetta.lock", &line)?;
    }
    Ok(file)
}

/// The built-in adapter ids: the ones `cadetta sync` is asked to run.
fn runnable(spec: &Value) -> Vec<String> {
    adapters(spec)
        .into_iter()
        .filter(|a| BUILTIN.contains(&a.as_str()))
        .collect()
}

/// Stage the fixture, sync it (attesting and syncing again when `attested`), observe O004.
pub fn run(spec: &Value) -> Result<Facts, String> {
    let scratch = Scratch::new()?;
    let root = &scratch.0;
    let file = {
        let _writing = writing();
        build(spec, root)?
    };
    let ids = runnable(spec);
    let sync = |label: &str| {
        let mut argv = vec!["sync".to_owned()];
        argv.extend(ids.iter().cloned());
        tracing::debug!(label, ?argv, "o004: running sync");
        run_cli(root, &argv)
    };
    let mut facts = sync("first")?;
    if spec["attested"].as_bool() == Some(true) {
        let role = spec["roles"]
            .as_object()
            .and_then(|r| r.keys().next().cloned())
            .unwrap_or_else(|| DEFAULT_ROLE.to_owned());
        let ack = run_cli(
            root,
            &[
                "ack".into(),
                format!("sync:{file}"),
                "--role".into(),
                role,
                "--reason".into(),
                "fixture attestation".into(),
                "--as".into(),
                "fixture@example.com".into(),
            ],
        )?;
        if ack.exit != 0 {
            return Err(format!(
                "ack sync:{file} failed: {}",
                ack.messages.join("; ")
            ));
        }
        facts = sync("after attest")?;
    }
    // The sync report omits a severity; O004 is info by definition (obligations.md 2.3).
    for p in facts.produced.iter_mut().filter(|p| p.code == "O004") {
        if p.severity.is_empty() {
            p.severity = "info".into();
        }
    }
    let open = facts.produced.iter().any(|p| p.code == "O004");
    tracing::info!(%file, open, "o004: judged");
    facts
        .observed
        .insert("obligation".into(), Value::from(OBLIGATION));
    facts.observed.insert(
        "state".into(),
        Value::from(if open { "open" } else { "current" }),
    );
    if open {
        let reason = cadetta_lint::rules::o004_w005::UNVERIFIABLE_REASON;
        if facts.messages.iter().any(|m| m.contains(reason)) {
            facts.observed.insert("reason".into(), Value::from(reason));
        }
    }
    Ok(facts)
}
