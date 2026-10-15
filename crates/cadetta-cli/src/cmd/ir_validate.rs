//! frob:ticket 01M4YMHQ5BQW8T67PVT5ZFM9K6
//! `cadetta ir-validate FILE`: check a `cadetta.ir/1` document against the committed JSON Schema
//! and the invariants of ir.md 6 (config-cli.md 3.9).
//!
//! Prints nothing on success and one `<json pointer>: <message>` line per violation. Exit 0
//! valid; 2 invalid, unreadable or not JSON. No diagnostic code: an invalid document is not a
//! design.

use std::io::Write;
use std::path::Path;

use serde_json::Value;

use crate::{EXIT_OK, EXIT_USAGE, Globals};

const USAGE: &str = "usage: cadetta ir-validate FILE";

/// Read FILE as JSON; the error is a one-line message.
pub fn load_json(file: &Path) -> Result<Value, String> {
    let text = std::fs::read_to_string(file)
        .map_err(|e| format!("{}: cannot read: {e}", file.display()))?;
    serde_json::from_str(&text).map_err(|e| format!("{}: not JSON: {e}", file.display()))
}

/// Every violation of `doc` (schema errors, then ir.md 6 invariants) as `<json pointer>: <message>`.
pub fn violations(doc: &Value) -> Vec<String> {
    let schema = cadetta_ir::schema::schema_value();
    let validator = jsonschema::validator_for(&schema).expect("the committed schema compiles");
    let mut found: Vec<String> = validator
        .iter_errors(doc)
        .map(|e| format!("{}: {e}", pointer_text(&e.instance_path().to_string())))
        .collect();
    found.extend(super::ir_invariants::check(doc));
    found.dedup();
    tracing::debug!(count = found.len(), "schema violations");
    found
}

/// The root pointer is the empty string; show it as `/` so every line has a pointer.
fn pointer_text(p: &str) -> &str {
    if p.is_empty() { "/" } else { p }
}

/// Run `cadetta ir-validate`; `args` are the words after the verb. Returns the exit code.
pub fn run_ir_validate(
    args: &[String],
    _globals: &Globals,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8 {
    let [file] = args else {
        tracing::error!(?args, "ir-validate needs exactly one FILE");
        let _ = writeln!(err, "error: {USAGE}");
        return EXIT_USAGE;
    };
    let doc = match load_json(Path::new(file)) {
        Ok(doc) => doc,
        Err(message) => {
            tracing::error!(%message, "ir-validate cannot load");
            let _ = writeln!(err, "error: {message}");
            return EXIT_USAGE;
        }
    };
    let found = violations(&doc);
    if found.is_empty() {
        tracing::info!(%file, "ir-validate: valid");
        return EXIT_OK;
    }
    tracing::warn!(%file, count = found.len(), "ir-validate: invalid");
    for line in &found {
        let _ = writeln!(out, "{line}");
    }
    EXIT_USAGE
}
