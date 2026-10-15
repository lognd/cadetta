//! frob:ticket 01M4VQXMH7YHJT18D9NJBHTKF3
//! `binding import`: the proposal an adapter's observed values produce (adapters.md 7.1, 7.2).
//!
//! The fixture's `items` become an adapter's `observed` JSON, which is decoded exactly as the
//! CLI decodes it and run through [`cadetta_adapter::import::propose::propose`]. Only the
//! propose path is modelled; accept, update and every item feature the library lacks (driven
//! excepted) fail loudly instead of passing.

use cadetta::cmd::import::decode_observed;
use cadetta_adapter::import::propose::{Status, propose};
use cadetta_ir::model::PackageId;
use serde_json::{Value, json};

use super::Job;
use crate::run::Facts;

/// Fixture keys this executor understands (`files` only restates the item files).
const MODELLED: &[&str] = &["adapter", "args", "items", "existing", "files"];

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

/// Run the fixture's `import` in propose mode and report the proposal's shape.
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
    let args: Vec<&str> = spec["args"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    if args.len() != 2 {
        return Err(format!(
            "import executor models only `ADAPTER FILE` (got {args:?})"
        ));
    }
    if spec["existing"].as_object().is_none_or(|o| !o.is_empty()) {
        return Err("import executor models an empty `existing` only".into());
    }
    let mut components = Vec::new();
    let mut bindings = Vec::new();
    for (n, item) in spec["items"].as_array().into_iter().flatten().enumerate() {
        let (member, binding) = observed_item(n, item)?;
        components.push(json!({"members": [member]}));
        bindings.push(binding);
    }
    let observed = json!({"components": components, "bindings": bindings});
    let id = PackageId {
        name: "fixture.pkg".into(),
        version: "0.1.0".into(),
    };
    let projects = decode_observed(&observed, "fixture", id, None)?;
    let proposal = propose(args[0], &projects);
    tracing::info!(
        items = proposal.items.len(),
        "import: fixture proposal built"
    );

    let mut facts = Facts::default();
    let mut put = |k: &str, v: Value| {
        facts.observed.insert(k.to_owned(), v);
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
    Ok(facts)
}
