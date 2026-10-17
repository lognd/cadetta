//! frob:ticket 01M4VQXMQ16RXW2ACNQHSDKA6V
//! Executors for the `binding` entries that drive the real `emit` and `impact --swap` code:
//! `emit` (scope binding), `emit-twice`, `emit-compare` and `impact-swap`.
//!
//! The fixture JSON describes the scenario; each executor builds a small package from it,
//! runs the CLI library on it and records what the expectation observes.

use std::fs;
use std::path::Path;

use cadetta_adapter::builtin::sw_equations;
use serde_json::{Value, json};

use crate::exec::Job;
use crate::run::{Facts, Scratch, parse_json, run_cli, snapshot, writing};

/// The emitted file every scenario writes (set by `out` in the generated `cadetta.toml`).
const OUT: &str = "build/eq.txt";

fn read_spec(job: &Job) -> Result<Value, String> {
    let text =
        fs::read_to_string(job.fixture).map_err(|e| format!("{}: {e}", job.fixture.display()))?;
    serde_json::from_str(&text).map_err(|e| format!("{}: {e}", job.fixture.display()))
}

fn put(root: &Path, rel: &str, text: &str) -> Result<(), String> {
    let _writing = writing();
    let path = root.join(rel);
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    fs::write(&path, text).map_err(|e| format!("{}: {e}", path.display()))
}

fn strings(spec: &Value, key: &str) -> Vec<String> {
    spec[key]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|v| v.as_str().map(str::to_owned))
        .collect()
}

/// `pkg::Model.member` split into its three parts.
fn split_path(path: &str) -> Result<(&str, &str, &str), String> {
    let (pkg, rest) = path
        .split_once("::")
        .ok_or_else(|| format!("path `{path}` lacks `::`"))?;
    let (model, member) = rest
        .split_once('.')
        .ok_or_else(|| format!("path `{path}` lacks `.member`"))?;
    Ok((pkg, model, member))
}

/// The `cadetta.toml` of the generated package: one `sw-equations` adapter with `extra` keys.
fn manifest(package: &str, deps: &[String], extra: &str) -> String {
    let mut text =
        format!("[package]\nname = \"{package}\"\nversion = \"0.1.0\"\nas_of = \"2026-10-13\"\n\n");
    if !deps.is_empty() {
        text.push_str("[dependencies]\n");
        for (i, dep) in deps.iter().enumerate() {
            text.push_str(&format!(
                "\"{dep}\" = {{ path = \"dep{i}\", version = \"1\" }}\n"
            ));
        }
        text.push('\n');
    }
    text.push_str(&format!(
        "[lint]\nfail_on = \"error\"\n\n[[adapter]]\nname = \"sw-equations\"\nout = \"{OUT}\"\n{extra}"
    ));
    text
}

/// Number of `"Name" = value` equations in the emitted file.
fn equation_count(text: &str) -> usize {
    text.lines().filter(|l| l.starts_with('"')).count()
}

/// `emit`: `bind = "scope"` emits a global per in-scope parameter, never a `let` or another package's member.
pub fn emit_scope(job: &Job) -> Result<Facts, String> {
    let spec = read_spec(job)?;
    let paths = strings(&spec, "paths");
    let lets = strings(&spec, "lets");
    let others = strings(&spec, "other_packages");
    let first = paths.first().ok_or("emit fixture lacks `paths`")?;
    let package = split_path(first)?.0.to_owned();

    let mut models: Vec<(String, Vec<String>)> = Vec::new();
    let mut add = |model: &str, line: String| match models.iter_mut().find(|(m, _)| m == model) {
        Some((_, lines)) => lines.push(line),
        None => models.push((model.to_owned(), vec![line])),
    };
    for p in &paths {
        let (_, model, member) = split_path(p)?;
        add(model, format!("  {member} = 10mm"));
    }
    for l in &lets {
        let (_, model, member) = split_path(l)?;
        add(model, format!("  let {member} = 2mm"));
    }
    // Another package's member is reachable only through a `let`, which is never emitted.
    let mut dep_names = Vec::new();
    let mut uses = String::new();
    let mut dep_models: Vec<(usize, String, Vec<String>)> = Vec::new();
    for (n, o) in others.iter().enumerate() {
        let (pkg, model, member) = split_path(o)?;
        if !dep_names.iter().any(|d| d == pkg) {
            dep_names.push(pkg.to_owned());
        }
        let idx = dep_names.iter().position(|d| d == pkg).unwrap_or(0);
        match dep_models
            .iter_mut()
            .find(|(i, m, _)| *i == idx && m == model)
        {
            Some((_, _, ms)) => ms.push(member.to_owned()),
            None => dep_models.push((idx, model.to_owned(), vec![member.to_owned()])),
        }
        uses.push_str(&format!("use {pkg}::{model}\n"));
        add("Probe", format!("  let other_{n} = {model}.{member}"));
    }

    let scratch = Scratch::new()?;
    let root = &scratch.0;
    put(
        root,
        "cadetta.toml",
        &manifest(&package, &dep_names, "bind = \"scope\"\n"),
    )?;
    let mut src = uses;
    for (model, lines) in &models {
        src.push_str(&format!("model {model} {{\n{}\n}}\n", lines.join("\n")));
    }
    put(root, "src/lib.cdt", &src)?;
    for (i, name) in dep_names.iter().enumerate() {
        put(
            root,
            &format!("dep{i}/cadetta.toml"),
            &format!("[package]\nname = \"{name}\"\nversion = \"1.0.0\"\nas_of = \"2026-10-13\"\n"),
        )?;
        let mut dsrc = String::new();
        for (_, model, members) in dep_models.iter().filter(|(d, _, _)| *d == i) {
            let body: Vec<String> = members.iter().map(|m| format!("  {m} = 3mm")).collect();
            dsrc.push_str(&format!("pub model {model} {{\n{}\n}}\n", body.join("\n")));
        }
        put(root, &format!("dep{i}/src/lib.cdt"), &dsrc)?;
    }

    let mut facts = run_cli(root, &["emit".into(), "sw-equations".into()])?;
    let count = facts.after.get(OUT).map_or(0, |t| equation_count(t));
    tracing::debug!(count, "emit: scope-bound equations");
    facts
        .observed
        .insert("emitted_globals".into(), json!(count));
    Ok(facts)
}

/// The IR digest token of an emitted file's header line, if any.
fn ir_digest(text: &str) -> Option<&str> {
    text.lines()
        .next()?
        .split_whitespace()
        .find(|t| t.starts_with("ir=blake3:"))
}

/// `emit-twice`: emit, change the named inputs, emit again; compare the bytes (adapters.md 4).
pub fn emit_twice(job: &Job) -> Result<Facts, String> {
    let spec = read_spec(job)?;
    let flag = |k: &str| spec[k].as_bool();
    let ir_changed = flag("ir_changed").unwrap_or_else(|| flag("inputs_same") == Some(false));
    let policy_changed = flag("policy_changed").unwrap_or(false);

    let source = |mm: u32| format!("model Plate {{\n  thickness = {mm}mm\n}}\n");
    let scratch = Scratch::new()?;
    let root = &scratch.0;
    put(
        root,
        "cadetta.toml",
        &manifest("acme.demo", &[], "bind = \"scope\"\nnative_unit = \"mm\"\n"),
    )?;
    put(root, "src/lib.cdt", &source(12))?;
    let first = run_cli(root, &["emit".into(), "sw-equations".into()])?;
    let first_text = first.after.get(OUT).cloned().unwrap_or_default();

    if ir_changed {
        put(root, "src/lib.cdt", &source(14))?;
    }
    if policy_changed {
        let toml = manifest("acme.demo", &[], "bind = \"scope\"\nnative_unit = \"cm\"\n");
        put(root, "cadetta.toml", &toml)?;
    }
    let mut facts = run_cli(root, &["emit".into(), "sw-equations".into()])?;
    facts.exit = facts.exit.max(first.exit);
    let second_text = facts.after.get(OUT).cloned().unwrap_or_default();

    let identical = !first_text.is_empty() && first_text == second_text;
    facts
        .observed
        .insert("byte_identical".into(), json!(identical));
    if !identical {
        let source = if ir_digest(&first_text) == ir_digest(&second_text) {
            "policy"
        } else {
            "ir"
        };
        facts.observed.insert("choice_source".into(), json!(source));
    }
    tracing::debug!(identical, ir_changed, policy_changed, "emit-twice compared");
    Ok(facts)
}

/// `emit-compare`: a file whose only difference from a fresh emit is the header is not rewritten (adapters.md 2).
pub fn emit_compare(job: &Job) -> Result<Facts, String> {
    let spec = read_spec(job)?;
    let field = |k: &str| {
        spec[k]
            .as_str()
            .map(str::to_owned)
            .ok_or_else(|| format!("emit-compare fixture lacks `{k}`"))
    };
    let (file, existing, fresh) = (
        field("existing_file")?,
        field("content")?,
        field("fresh_content")?,
    );
    let scratch = Scratch::new()?;
    put(&scratch.0, &file, &existing)?;
    let mut facts = Facts {
        before: snapshot(&scratch.0),
        ..Facts::default()
    };
    if !sw_equations::same_modulo_header(&existing, &fresh) {
        put(&scratch.0, &file, &fresh)?;
    }
    facts.after = snapshot(&scratch.0);
    let rewritten = facts.before != facts.after;
    facts
        .observed
        .insert("file_rewritten".into(), json!(rewritten));
    Ok(facts)
}

/// `impact-swap`: `impact --swap` lists replace-component actions for an assembly's slot.
pub fn impact_swap(job: &Job) -> Result<Facts, String> {
    let spec = read_spec(job)?;
    let field = |k: &str| {
        spec[k]
            .as_str()
            .map(str::to_owned)
            .ok_or_else(|| format!("impact-swap fixture lacks `{k}`"))
    };
    let (slot, primary, alternate) = (field("slot")?, field("primary")?, field("alternate")?);
    let bound = strings(&spec, "bindings_for");
    let (package, iface) = slot.split_once("::").ok_or("slot lacks `::`")?;
    let bare = |p: &str| p.rsplit("::").next().unwrap_or(p).to_owned();
    let (primary_name, alternate_name) = (bare(&primary), bare(&alternate));

    let mut src = format!("interface {iface} {{\n  diameter: Length\n}}\n");
    let mut names = vec![primary_name.clone()];
    if alternate_name != primary_name {
        names.push(alternate_name.clone());
    }
    for (i, n) in names.iter().enumerate() {
        src.push_str(&format!(
            "ots {n}: {iface} {{\n  diameter = {}mm\n}}\n",
            100 - i * 10
        ));
    }
    src.push_str(&format!(
        "assembly Rover {{\n  part wheel: {iface} = {primary_name} alt [{}]\n}}\n",
        if alternate_name == primary_name {
            String::new()
        } else {
            alternate_name.clone()
        }
    ));

    let mut bindings = String::new();
    bindings.push_str("[\"");
    bindings.push_str(package);
    bindings.push_str("::Rover/wheel.diameter\"]\nlocators = [\n");
    bindings.push_str("  { locator = \"Wheel.SLDPRT@Rover.SLDASM\", kind = \"dimension\", mode = \"value\", native_unit = \"mm\", component = \"wheel<2>\" },\n");
    for comp in &bound {
        bindings.push_str(&format!(
            "  {{ locator = \"D1@Bore@{n}.SLDPRT\", kind = \"dimension\", mode = \"value\", native_unit = \"mm\", for = \"{comp}\" }},\n",
            n = bare(comp)
        ));
    }
    bindings.push_str("]\n");

    let scratch = Scratch::new()?;
    let root = &scratch.0;
    put(root, "cadetta.toml", &manifest(package, &[], ""))?;
    put(root, "src/lib.cdt", &src)?;
    put(root, "bindings/sw-equations.toml", &bindings)?;
    let args = [
        "impact".to_owned(),
        "--swap".into(),
        format!("{package}::Rover/wheel"),
        alternate.clone(),
    ];
    let mut facts = run_cli(root, &args)?;
    let actions: Vec<Value> = parse_json(&facts.stdout)
        .map(|r| {
            r["cad_resync"]["replace_component"]
                .as_array()
                .map(|a| a.iter().map(|_| json!("replace-component")).collect())
                .unwrap_or_default()
        })
        .unwrap_or_default();
    facts
        .observed
        .insert("actions".into(), Value::Array(actions));
    Ok(facts)
}

/// `{ "call": "display", "value", "spec" }`: run `cadetta display` and observe the printed `result`.
pub fn display(job: &Job) -> Result<Facts, String> {
    let spec = read_spec(job)?;
    let call = spec["call"].as_str().unwrap_or_default();
    if call != "display" {
        return Err(format!("binding call `{call}` has no executor"));
    }
    let field = |key: &str| {
        spec[key]
            .as_str()
            .map(str::to_owned)
            .ok_or_else(|| format!("display call lacks string `{key}`"))
    };
    let args = vec!["display".to_owned(), field("value")?, field("spec")?];
    let scratch = Scratch::new()?;
    let mut facts = run_cli(&scratch.0, &args)?;
    if let Some(text) =
        parse_json(&facts.stdout).and_then(|v| v["text"].as_str().map(str::to_owned))
    {
        facts.observed.insert("result".into(), json!(text));
    }
    tracing::debug!(?args, exit = facts.exit, "conformance: binding display");
    Ok(facts)
}
