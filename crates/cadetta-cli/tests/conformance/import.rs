//! frob:ticket 01M4VQXMH7YHJT18D9NJBHTKF3
//! frob:ticket 01M4ZBN0XKQRTXW7AY1KKVE13B
//! `binding import`: propose, accept, re-run and `--update` (adapters.md 7.1, 7.2, 7.4).
//!
//! The fixture's `items` become an adapter's `observed` JSON. Propose-shaped facts come from
//! decoding it exactly as the CLI does and running
//! [`cadetta_adapter::import::propose::propose`]. File-level facts (writes, lock adoption,
//! re-run, update diffs and edited sources) come from running the real `cadetta import` against
//! a scratch package whose adapter is a scripted shell program replaying that `observed` JSON.
//! Item features the library lacks fail loudly instead of passing. `then_sync` classifies each
//! updated locator the way the next `sync` would (lock base after the run, source value, native
//! value), observed as `next_sync_class` (and `next_run_class` for a single locator).
//!
//! Executor conventions: `cadetta.lock` is left out of the `writes` view (the lock is observed
//! through `lock_adds`); `--accept` fixtures run propose first, so the proposal files are not
//! counted as accept writes; `existing` bindings with no `--accept` mean a re-run, i.e. propose
//! then accept against those bindings.

use cadetta::cmd::import::decode_observed;
use cadetta_adapter::classify::classify;
use cadetta_adapter::import::naming::{component_name, member_name};
use cadetta_adapter::import::propose::{Status, propose};
use cadetta_ir::model::PackageId;
use serde_json::{Value, json};

use std::collections::BTreeMap;

use super::protocol::{put, put_exec, rpc};
use super::{Job, class_label};
use crate::run::{Facts, Scratch, parse_json, run_cli, writing};

/// Fixture keys this executor understands (`files` only restates the item files).
const MODELLED: &[&str] = &[
    "adapter",
    "args",
    "items",
    "existing",
    "files",
    "lock_base",
    "source",
    "then_sync",
];

/// Package name of the scratch package the CLI runs against.
const PACKAGE: &str = "acme.plate";

/// Item keys this executor understands.
const MODELLED_ITEM: &[&str] = &["file", "locator", "kind", "number", "unit", "driven"];

/// One fixture item as the adapter's `observed` JSON (member plus binding), or why it cannot.
fn observed_item(n: usize, item: &Value) -> Result<(Value, Value), String> {
    if let Some(k) = item
        .as_object()
        .into_iter()
        .flat_map(|o| o.keys())
        .find(|k| !MODELLED_ITEM.contains(&k.as_str()))
    {
        return Err(format!("import executor does not model item key `{k}`"));
    }
    let text = |k: &str| item[k].as_str().map(str::to_owned);
    let (Some(file), Some(locator), Some(kind)) = (text("file"), text("locator"), text("kind"))
    else {
        return Err("item lacks file, locator or kind".into());
    };
    let number = item["number"]
        .as_i64()
        .ok_or("import executor models integer `number` only")?;
    let unit = text("unit").ok_or("import executor models items with a `unit` only")?;
    let metres = match unit.as_str() {
        "mm" => format!("{number}/1000"),
        "m" => format!("{number}/1"),
        other => return Err(format!("import executor does not model unit `{other}`")),
    };
    let path = format!("fixture.pkg::Observed.{n}");
    let member = json!({
        "path": path,
        "value": {"nominal": {"r": metres, "pi": 0}, "dim": {"m": 1},
                   "authored": number.to_string(), "exact": true},
        "provenance": {"imported": {"adapter": "fixture", "file": file, "native_name": locator}},
    });
    let mode = if item["driven"].as_bool() == Some(true) {
        "ref"
    } else {
        "value"
    };
    let binding = json!({"path": path, "adapter": "fixture", "locators": [
        {"locator": locator, "kind": kind, "mode": mode, "native_unit": unit}]});
    Ok((member, binding))
}

/// Run the fixture's `import`: propose-shaped facts in process, file-level facts through the CLI.
pub fn run(job: &Job) -> Result<Facts, String> {
    let text = std::fs::read_to_string(job.fixture)
        .map_err(|e| format!("{}: {e}", job.fixture.display()))?;
    let spec: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    if let Some(k) = spec
        .as_object()
        .into_iter()
        .flat_map(|o| o.keys())
        .find(|k| !MODELLED.contains(&k.as_str()))
    {
        return Err(format!("import executor does not model fixture key `{k}`"));
    }
    let args: Vec<String> = spec["args"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|v| v.as_str().map(str::to_owned))
        .collect();
    let positional: Vec<String> = args
        .iter()
        .filter(|a| !a.starts_with("--"))
        .cloned()
        .collect();
    if positional.len() != 2 {
        return Err(format!(
            "import executor models only `ADAPTER FILE` (got {args:?})"
        ));
    }
    let flag = |f: &str| args.iter().any(|a| a == f);
    let mut components = Vec::new();
    let mut bindings = Vec::new();
    for (n, item) in spec["items"].as_array().into_iter().flatten().enumerate() {
        let (member, binding) = observed_item(n, item)?;
        components.push(json!({"members": [member]}));
        bindings.push(binding);
    }
    let observed = json!({"components": components, "bindings": bindings});
    let update = flag("--update");
    let mut observations = BTreeMap::new();
    if !update {
        observations = proposal_facts(&spec, &positional[0], &observed)?;
    }
    let scratch = Scratch::new()?;
    let root = &scratch.0;
    stage(&spec, &observed, root)?;
    touch_input(root, &positional[1])?;
    let mut facts = if update {
        let write = flag("--write");
        let mut argv = vec![
            "import".to_owned(),
            positional[0].clone(),
            positional[1].clone(),
        ];
        argv.push("--update".into());
        if write {
            argv.push("--write".into());
        }
        let facts = run_cli(root, &argv)?;
        update_facts(&spec, facts)
    } else {
        let accept = flag("--accept");
        let rerun = !accept && has_existing_bindings(&spec);
        let propose_args = vec![
            "import".to_owned(),
            positional[0].clone(),
            positional[1].clone(),
        ];
        let facts = run_cli(root, &propose_args)?;
        if accept || rerun {
            if facts.exit != 0 {
                return Err(format!("propose failed: {}", facts.stderr));
            }
            let mut argv = propose_args;
            argv.push("--accept".into());
            let facts = run_cli(root, &argv)?;
            accept_facts(&spec, facts)
        } else {
            plain_facts(facts)
        }
    };
    facts.observed.extend(observations);
    Ok(facts)
}

/// Create the (empty) CAD file argument and a bare `src/` unless already staged.
fn touch_input(root: &std::path::Path, file: &str) -> Result<(), String> {
    let _writing = writing();
    if !root.join("src").exists() {
        put(root, "src/lib.cdt", "")?;
    }
    if root.join(file).exists() {
        return Ok(());
    }
    put(root, file, "")
}

/// Whether `existing` carries pre-bound locators (a re-run).
fn has_existing_bindings(spec: &Value) -> bool {
    spec["existing"]
        .as_object()
        .is_some_and(|o| o.values().any(Value::is_array))
}

/// Write the scratch package: manifest, scripted adapter, `existing` files and update sources.
fn stage(spec: &Value, observed: &Value, root: &std::path::Path) -> Result<(), String> {
    let id = spec["adapter"]["id"]
        .as_str()
        .ok_or("fixture lacks adapter.id")?;
    let unit = spec["adapter"]["native_unit"].as_str().unwrap_or("mm");
    let init = rpc(
        1,
        &json!({"name": id, "version": "1.0", "capabilities": ["emit", "read"],
                "native_units": unit, "precision": 6, "platforms": ["linux"]}),
    );
    let read = rpc(2, &json!({"observed": observed, "diagnostics": []}));
    let script = format!(
        "#!/bin/sh\nread line\nprintf '%s\\n' '{init}'\nread line\nprintf '%s\\n' '{read}'\nread line\nprintf '%s\\n' '{}'\n",
        rpc(3, &Value::Null)
    );
    let _writing = writing();
    put(
        root,
        "cadetta.toml",
        &format!(
            "[package]\nname = \"{PACKAGE}\"\nversion = \"0.1.0\"\n[[adapter]]\nname = \"{id}\"\nversion = \"1.0\"\ncommand = [\"adapter.sh\"]\n"
        ),
    )?;
    put_exec(root, "adapter.sh", &script)?;
    let items = spec["items"].as_array().cloned().unwrap_or_default();
    for (path, content) in spec["existing"].as_object().into_iter().flatten() {
        match content {
            Value::String(text) => put(root, path, text)?,
            Value::Array(locators) => {
                let mut table = String::new();
                for locator in locators.iter().filter_map(Value::as_str) {
                    let item = items
                        .iter()
                        .find(|i| i["locator"].as_str() == Some(locator))
                        .ok_or_else(|| format!("existing locator `{locator}` is not an item"))?;
                    let canonical = format!(
                        "{PACKAGE}::{}.{}",
                        component_name(item["file"].as_str().unwrap_or_default()),
                        member_name(locator)
                    );
                    table.push_str(&binding_table(&canonical, locator, item));
                }
                put(root, path, &table)?;
            }
            other => return Err(format!("existing `{path}`: unsupported value {other}")),
        }
    }
    if spec.get("source").is_some() || spec.get("lock_base").is_some() {
        stage_update(spec, &items, root)?;
    }
    tracing::info!(adapter = id, "import: scratch package staged");
    Ok(())
}

/// One binding table for `locator` of `item`, at `canonical`.
fn binding_table(canonical: &str, locator: &str, item: &Value) -> String {
    format!(
        "[\"{canonical}\"]\nlocators = [ {{ locator = \"{locator}\", kind = \"{}\", mode = \"value\", native_unit = \"{}\" }} ]\n",
        item["kind"].as_str().unwrap_or("global"),
        item["unit"].as_str().unwrap_or("mm"),
    )
}

/// Stage the `--update` package: bound `Part` members, their `.cdt` declarations and lock bases.
fn stage_update(spec: &Value, items: &[Value], root: &std::path::Path) -> Result<(), String> {
    let id = spec["adapter"]["id"].as_str().unwrap_or_default();
    let sources = spec["source"].as_object().cloned().unwrap_or_default();
    let mut bindings = String::new();
    let mut lock = String::new();
    for item in items {
        let name = item["locator"].as_str().unwrap_or_default();
        let canonical = format!("{PACKAGE}::Part.{name}");
        bindings.push_str(&binding_table(&canonical, name, item));
        if let Some(base) = spec["lock_base"][name].as_str() {
            lock.push_str(&format!(
                "synced {id} {canonical} locator={name} value={base}\n"
            ));
        }
    }
    let mut cites = String::new();
    let mut body = String::new();
    let defined: Vec<&str> = sources.keys().map(String::as_str).collect();
    for (name, src) in &sources {
        let src = src.as_str().ok_or("source values must be strings")?;
        let (decl, cite) = declaration(name, src);
        if let Some(cite) = cite {
            cites.push_str(&format!(
                "cite {cite}: document {{\n  file = \"docs/{cite}.txt\"\n  title = \"{cite}\"\n  date = \"2026-01-01\"\n}}\n"
            ));
        }
        body.push_str(&format!("  {decl}\n"));
        for ident in decl
            .split_once('=')
            .map(|(_, rhs)| rhs)
            .unwrap_or_default()
            .split(|c: char| !c.is_alphanumeric() && c != '_')
            .filter(|w| w.chars().next().is_some_and(char::is_alphabetic))
        {
            if decl.starts_with("let ")
                && !defined.contains(&ident)
                && !body.contains(&format!("  {ident} = "))
            {
                body.push_str(&format!("  {ident} = 1mm\n"));
            }
        }
    }
    put(root, &format!("bindings/{id}.toml"), &bindings)?;
    put(root, "cadetta.lock", &lock)?;
    put(
        root,
        "src/lib.cdt",
        &format!("{cites}model Part {{\n{body}}}\n"),
    )
}

/// The `.cdt` declaration for fixture source `src` of member `name`, plus a cite id when cited.
fn declaration(name: &str, src: &str) -> (String, Option<String>) {
    if src.contains('=') {
        return (src.to_owned(), None);
    }
    let (value, cite) = match src.split_once(" cite ") {
        Some((v, c)) => (v, Some(c.rsplit('.').next().unwrap_or(c).to_owned())),
        None => (src, None),
    };
    let value = match value
        .split_once(" +")
        .and_then(|(v, r)| Some((v, r.split_once("/-")?)))
    {
        Some((v, (plus, minus))) => format!("{v} [+{plus} -{minus}]"),
        None => value.to_owned(),
    };
    let tail = cite
        .as_ref()
        .map(|c| format!(" @{c}:\"1\""))
        .unwrap_or_default();
    (format!("{name} = {value}{tail}"), cite)
}

/// The fixture's view of the member `name` in the rewritten source.
fn source_view(after: &str, name: &str, full: bool) -> Option<String> {
    let line = after.lines().map(str::trim).find(|l| {
        l.strip_prefix("let ")
            .unwrap_or(l)
            .strip_prefix(name)
            .is_some_and(|r| r.trim_start().starts_with('='))
    })?;
    if full {
        return Some(line.to_owned());
    }
    let rhs = line.split_once('=')?.1.trim();
    let rhs = rhs.split(" @").next().unwrap_or(rhs);
    Some(
        match rhs
            .split_once(" [+")
            .and_then(|(v, t)| Some((v, t.strip_suffix(']')?)))
        {
            Some((v, tol)) => format!("{v} +{}", tol.replace(" -", "/-")),
            None => rhs.to_owned(),
        },
    )
}

/// The report object a CLI run printed (`{}` when stdout is not JSON).
fn report_of(facts: &Facts) -> Value {
    parse_json(&facts.stdout).unwrap_or(Value::Null)
}

fn strings_of(v: &Value) -> Vec<String> {
    v.as_array()
        .into_iter()
        .flatten()
        .filter_map(|s| s.as_str().map(str::to_owned))
        .collect()
}

/// Drop `cadetta.lock` from the write view; the lock is observed through `lock_adds`.
fn without_lock(facts: &mut Facts) -> (String, String) {
    let before = facts.before.remove("cadetta.lock").unwrap_or_default();
    let after = facts.after.remove("cadetta.lock").unwrap_or_default();
    (before, after)
}

/// Observations shared by every mode: binding and source writes, adopted records.
fn file_facts(facts: &mut Facts) {
    let (lock_before, lock_after) = without_lock(facts);
    let changed: Vec<&String> = facts
        .after
        .iter()
        .filter(|(p, t)| facts.before.get(*p) != Some(t))
        .map(|(p, _)| p)
        .collect();
    let bindings = changed.iter().any(|p| p.starts_with("bindings/"));
    let source = changed
        .iter()
        .any(|p| p.ends_with(".cdt") && !p.starts_with("build/") && facts.before.contains_key(*p));
    let adds: Vec<&str> = lock_after
        .lines()
        .filter(|l| l.starts_with("adopted ") && !lock_before.lines().any(|b| b == *l))
        .collect();
    let adds = json!(adds);
    facts
        .observed
        .insert("writes_bindings".into(), json!(bindings));
    facts.observed.insert("writes_source".into(), json!(source));
    facts
        .observed
        .insert("source_modified".into(), json!(source));
    facts.observed.insert("lock_adds".into(), adds);
}

/// Facts of a plain propose run.
fn plain_facts(mut facts: Facts) -> Facts {
    file_facts(&mut facts);
    facts
}

/// Facts of an accept (or re-run) run: what was added and what stayed, as native locators.
fn accept_facts(spec: &Value, mut facts: Facts) -> Facts {
    file_facts(&mut facts);
    let report = report_of(&facts);
    let locators: Vec<&str> = spec["items"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|i| i["locator"].as_str())
        .collect();
    let named = |canonical: &str| {
        let member = canonical.rsplit('.').next().unwrap_or(canonical);
        locators
            .iter()
            .find(|l| member_name(l) == member)
            .map(|l| (*l).to_owned())
    };
    let proposed: Vec<String> = strings_of(&report["items"])
        .iter()
        .filter_map(|c| named(c))
        .collect();
    let unchanged: Vec<String> = strings_of(&report["unchanged"])
        .iter()
        .filter(|c| c.contains("::"))
        .filter_map(|c| named(c))
        .collect();
    // File paths in `unchanged` are files left alone; `overwrote` is a pre-existing file whose
    // text changed (never expected: adapters.md 7.1).
    let refused: Vec<String> = strings_of(&report["unchanged"])
        .into_iter()
        .filter(|c| !c.contains("::"))
        .collect();
    let overwrote: Vec<&String> = spec["existing"]
        .as_object()
        .into_iter()
        .flatten()
        .filter(|(_, v)| v.is_string())
        .map(|(p, _)| p)
        .filter(|p| facts.after.get(*p) != facts.before.get(*p))
        .collect();
    facts.observed.insert("overwrote".into(), json!(overwrote));
    facts.observed.insert("refused".into(), json!(refused));
    facts.observed.insert("proposed".into(), json!(proposed));
    facts.observed.insert("unchanged".into(), json!(unchanged));
    facts
}

/// Facts of an `--update` run: diff lines, reported skips, citation flags and the edited sources.
fn update_facts(spec: &Value, mut facts: Facts) -> Facts {
    let lock_after = facts.after.get("cadetta.lock").cloned().unwrap_or_default();
    file_facts(&mut facts);
    let report = report_of(&facts);
    let (mut diff, mut reported, mut flagged) = (Vec::new(), Vec::new(), Vec::new());
    for block in strings_of(&report["items"]) {
        let mut name = String::new();
        for line in block.lines() {
            if let Some(rest) = line.strip_prefix("- ").or_else(|| line.strip_prefix("+ ")) {
                diff.push(format!("{} {name} = {rest}", &line[..1]));
            } else if let Some((path, why)) = line.split_once(": ") {
                let member = path.rsplit('.').next().unwrap_or(path);
                if why == "value no longer matches its cite" {
                    flagged.push(why.to_owned());
                } else if let Some(v) = why
                    .strip_prefix("skipped, has a tolerance or fit; CAD value ")
                    .and_then(|r| r.strip_suffix(", edit by hand"))
                {
                    reported.push(format!("{member}: observed {v}, edit by hand"));
                } else if let Some(why) = why.strip_prefix("skipped, ") {
                    reported.push(why.to_owned());
                }
            } else if let Some((path, _)) = line.split_once(" (") {
                name = path.rsplit('.').next().unwrap_or(path).to_owned();
            }
        }
    }
    let after = facts.after.get("src/lib.cdt").cloned().unwrap_or_default();
    let mut source_after = serde_json::Map::new();
    for (name, src) in spec["source"].as_object().into_iter().flatten() {
        let full = src.as_str().is_some_and(|s| s.contains('='));
        if let Some(v) = source_view(&after, name, full) {
            source_after.insert(name.clone(), json!(v));
        }
    }
    let before = facts.before.get("src/lib.cdt").cloned().unwrap_or_default();
    let mut edits = Vec::new();
    for (name, src) in spec["source"].as_object().into_iter().flatten() {
        let full = src.as_str().is_some_and(|s| s.contains('='));
        let (was, now) = (
            source_view(&before, name, full),
            source_view(&after, name, full),
        );
        if was != now
            && let Some(now) = now
        {
            edits.push(format!("{name} = {now}"));
        }
    }
    facts.observed.insert("source_edits".into(), json!(edits));
    if spec["then_sync"].as_bool() == Some(true) {
        next_sync(spec, &lock_after, &after, &mut facts);
    }
    facts.observed.insert("diff".into(), json!(diff));
    facts.observed.insert("reported".into(), json!(reported));
    facts.observed.insert("flagged".into(), json!(flagged));
    facts
        .observed
        .insert("source_after".into(), Value::Object(source_after));
    facts
}

/// The `value=` of the lock's `synced` record for `locator`, when there is one.
fn lock_value(lock: &str, locator: &str) -> Option<String> {
    lock.lines()
        .filter(|l| l.starts_with("synced "))
        .find(|l| {
            l.contains(&format!(" locator={locator} "))
                || l.contains(&format!(" locator={locator}\t"))
        })
        .and_then(|l| l.split_once(" value="))
        .map(|(_, v)| v.trim().to_owned())
}

/// Observe the class the next `sync` gives each item: lock base after the run, the source value
/// now, and the CAD value (`then_sync`; adapters.md 7.4).
fn next_sync(spec: &Value, lock_after: &str, source_after: &str, facts: &mut Facts) {
    let mut classes = serde_json::Map::new();
    for item in spec["items"].as_array().into_iter().flatten() {
        let name = item["locator"].as_str().unwrap_or_default();
        let full = spec["source"][name]
            .as_str()
            .is_some_and(|s| s.contains('='));
        let ours = source_view(source_after, name, full).unwrap_or_default();
        let theirs = format!(
            "{}{}",
            item["number"],
            item["unit"].as_str().unwrap_or_default()
        );
        let base = lock_value(lock_after, name);
        let class = classify(base.as_deref(), &ours, &theirs);
        tracing::info!(name, ?base, %ours, %theirs, ?class, "import: next sync classified");
        classes.insert(name.to_owned(), json!(class_label(class)));
    }
    if let [(_, only)] = classes.iter().collect::<Vec<_>>().as_slice() {
        facts
            .observed
            .insert("next_run_class".into(), (*only).clone());
        facts
            .observed
            .insert("next_sync_class".into(), (*only).clone());
    }
    facts
        .observed
        .insert("next_sync_classes".into(), Value::Object(classes));
}

/// Propose-shaped facts: components, members, flags, order and per-locator status.
fn proposal_facts(
    spec: &Value,
    adapter: &str,
    observed: &Value,
) -> Result<BTreeMap<String, Value>, String> {
    let id = PackageId {
        name: "fixture.pkg".into(),
        version: "0.1.0".into(),
    };
    let projects = decode_observed(observed, "fixture", id, None)?;
    let proposal = propose(adapter, &projects);
    tracing::info!(
        items = proposal.items.len(),
        "import: fixture proposal built"
    );
    let mut out = BTreeMap::new();
    let mut put = |k: &str, v: Value| {
        out.insert(k.to_owned(), v);
    };
    let mut comps: Vec<&str> = Vec::new();
    for i in &proposal.items {
        if !comps.contains(&i.component.as_str()) {
            comps.push(&i.component);
        }
    }
    put("components", json!(comps));
    // Member names are allocated in observation order; the proposal itself is sorted.
    let mut allocated: Vec<&_> = proposal.items.iter().collect();
    let position = |file: &str, locator: &str| {
        spec["items"]
            .as_array()
            .into_iter()
            .flatten()
            .position(|it| {
                it["file"].as_str() == Some(file) && it["locator"].as_str() == Some(locator)
            })
    };
    allocated.sort_by_key(|i| position(&i.file, &i.locator));
    put(
        "members",
        json!(allocated.iter().map(|i| &i.path).collect::<Vec<_>>()),
    );
    put(
        "flagged",
        json!(
            allocated
                .iter()
                .filter(|i| i.status == Status::Accept && i.reason.is_some())
                .map(|i| &i.path)
                .collect::<Vec<_>>()
        ),
    );
    put(
        "item_order",
        json!(
            proposal
                .items
                .iter()
                .map(|i| format!("{}:{}", i.file, i.locator))
                .collect::<Vec<_>>()
        ),
    );
    let status: serde_json::Map<String, Value> = proposal
        .items
        .iter()
        .map(|i| {
            let s = if i.status == Status::Skip {
                "skip"
            } else {
                "accept"
            };
            (i.locator.clone(), json!(s))
        })
        .collect();
    put("status", Value::Object(status));
    put(
        "reason_present",
        json!(
            proposal
                .items
                .iter()
                .any(|i| i.status == Status::Skip && i.reason.is_some())
        ),
    );
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::lock_value;

    // frob:tests crates/cadetta-cli/tests/conformance/import.rs::lock_value
    #[test]
    fn lock_value_reads_the_synced_record_for_a_locator() {
        let lock = "synced solidworks acme::Part.width locator=width value=11mm\n";
        assert_eq!(lock_value(lock, "width").as_deref(), Some("11mm"));
        assert_eq!(lock_value(lock, "depth"), None);
    }
}
