//! frob:ticket 01M4YRD0AKPB6JF7X2D1PKQ63N
//! `compare digest` and `compare classify`: two inputs, one verdict (ir.md 4, ir.md 5).
//!
//! A digest fixture stages side `a` and side `b` as packages, reads `cadetta ir` for each and
//! reports whether the named node's own `digest` or Merkle `closure` is the `same` or `differs`.
//! A classify fixture feeds two JSON schema documents to [`cadetta_ir::schema::classify`] and
//! reports `none`, `minor` or `major`. The verdicts land in `observed` under `digests` and
//! `classify`, where the generic verifier compares them with the block.

use std::path::{Path, PathBuf};

use cadetta_ir::schema::{Change, classify};
use serde_json::{Value, json};

use super::Job;
use crate::run::{Facts, parse_json, run_cli, stage};

/// Wire label of a schema change (`Breaking` is spelled `major` on the wire).
fn label(change: Change) -> &'static str {
    match change {
        Change::None => "none",
        Change::Minor => "minor",
        Change::Breaking => "major",
    }
}

/// The path of side `side` (`a` or `b`) of the fixture stem, with `ext` or as a package directory.
fn side(stem: &Path, side: &str, ext: &str) -> Result<PathBuf, String> {
    let name = stem
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("compare fixture has no name")?;
    let dir = stem.parent().ok_or("compare fixture has no directory")?;
    [format!("{name}.{side}.{ext}"), format!("{name}.{side}")]
        .iter()
        .map(|n| dir.join(n))
        .find(|p| p.exists())
        .ok_or_else(|| format!("compare fixture lacks side `{side}` (`{name}.{side}.{ext}`)"))
}

/// Every object under `value` whose `path` is `path` and that carries `field`.
fn find_field<'a>(value: &'a Value, path: &str, field: &str, out: &mut Vec<&'a Value>) {
    match value {
        Value::Object(map) => {
            if map.get("path").and_then(Value::as_str) == Some(path)
                && let Some(v) = map.get(field)
            {
                out.push(v);
            }
            map.values().for_each(|v| find_field(v, path, field, out));
        }
        Value::Array(items) => items.iter().for_each(|v| find_field(v, path, field, out)),
        _ => {}
    }
}

/// The `field` (`digest` or `closure`) of the node at `path` in an IR document.
fn node_field(ir: &Value, path: &str, field: &str) -> Option<String> {
    let mut found = Vec::new();
    find_field(ir, path, field, &mut found);
    found.first().and_then(|v| v.as_str()).map(str::to_owned)
}

/// The `cadetta ir` document of the package at `source` (a `.cdt` file or a package directory).
fn ir_of(source: &Path) -> Result<Value, String> {
    let scratch = stage(source)?;
    let facts = run_cli(&scratch.0, &["ir".to_owned()])?;
    if facts.exit != 0 {
        return Err(format!(
            "`cadetta ir` failed on {} (exit {}): {}",
            source.display(),
            facts.exit,
            facts.messages.join("; ")
        ));
    }
    parse_json(&facts.stdout)
        .ok_or_else(|| format!("`cadetta ir` on {} printed no JSON", source.display()))
}

/// Verdict for one `digests` row: `same` when both sides carry equal values, else `differs`.
fn relation(a: &Value, b: &Value, row: &Value) -> Result<&'static str, String> {
    let path = row["path"].as_str().ok_or("digests row lacks `path`")?;
    let field = row["field"].as_str().unwrap_or("closure");
    let key = match field {
        "own" => "digest",
        "closure" => "closure",
        other => {
            return Err(format!(
                "digests row `field` must be own or closure, got `{other}`"
            ));
        }
    };
    let side = |ir: &Value, which: &str| {
        node_field(ir, path, key)
            .ok_or_else(|| format!("no node `{path}` with a {key} on side {which}"))
    };
    let (x, y) = (side(a, "a")?, side(b, "b")?);
    let verdict = if x == y { "same" } else { "differs" };
    tracing::debug!(path, field, verdict, "compared digests");
    Ok(verdict)
}

/// `compare digest`: whether each named node's digest or closure changes between sides `a` and `b`.
pub fn digest(job: &Job) -> Result<Facts, String> {
    let rows = job
        .block
        .extra
        .get("digests")
        .and_then(Value::as_array)
        .ok_or("digest fixture needs a `digests` list")?;
    let a = ir_of(&side(job.fixture, "a", "cdt")?)?;
    let b = ir_of(&side(job.fixture, "b", "cdt")?)?;
    let mut observed = Vec::new();
    for row in rows {
        let mut row = row.clone();
        row["relation"] = json!(relation(&a, &b, &row)?);
        observed.push(row);
    }
    let mut facts = Facts::default();
    facts
        .observed
        .insert("digests".into(), Value::Array(observed));
    Ok(facts)
}

/// `compare classify`: the schema change class from side `a` (old) to side `b` (new).
pub fn classify_schema(job: &Job) -> Result<Facts, String> {
    let load = |which: &str| -> Result<Value, String> {
        let path = side(job.fixture, which, "json")?;
        let text =
            std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
    };
    let verdict = label(classify(&load("a")?, &load("b")?));
    tracing::debug!(verdict, "classified schema change");
    let mut facts = Facts::default();
    facts.observed.insert("classify".into(), json!(verdict));
    Ok(facts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn label_spells_breaking_as_major() {
        assert_eq!(label(Change::Breaking), "major");
        assert_eq!(label(Change::Minor), "minor");
        assert_eq!(label(Change::None), "none");
    }

    #[test]
    fn node_field_finds_nested_path() {
        let ir = json!({"components": [{"members": [{"path": "p::X.a", "digest": "d1", "closure": "c1"}]}]});
        assert_eq!(node_field(&ir, "p::X.a", "closure").as_deref(), Some("c1"));
        assert_eq!(node_field(&ir, "p::X.b", "closure"), None);
    }

    #[test]
    fn relation_distinguishes_same_from_differs() {
        let mk = |c: &str| json!({"members": [{"path": "p::X.a", "digest": "d", "closure": c}]});
        let row = json!({"path": "p::X.a", "field": "closure"});
        assert_eq!(relation(&mk("c1"), &mk("c1"), &row), Ok("same"));
        assert_eq!(relation(&mk("c1"), &mk("c2"), &row), Ok("differs"));
        let own = json!({"path": "p::X.a", "field": "own"});
        assert_eq!(relation(&mk("c1"), &mk("c2"), &own), Ok("same"));
    }
}
