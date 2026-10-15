//! frob:ticket 01M4YEA3QBT5257Z8BAN7NH9KY
//! An interface bound declared in source reaches `cadetta check`: a violating implementer
//! reports C001, a satisfying one does not (lint_facts::interfaces lowers the bounds).

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

/// Stages the language fixture `name` as a one-file package.
fn stage(name: &str) -> PathBuf {
    let src = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/conformance/language")
        .join(format!("{name}.cdt"));
    let dir = std::env::temp_dir().join(format!("cadetta-ibc-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("src")).expect("mkdir");
    fs::write(
        dir.join("cadetta.toml"),
        "[package]\nname = \"conf\"\nversion = \"0.1.0\"\nas_of = \"2026-10-13\"\n\n[lint]\nfail_on = \"error\"\n",
    )
    .expect("manifest");
    fs::copy(src, dir.join("src/lib.cdt")).expect("copy");
    dir
}

/// Every diagnostic code `cadetta check` reports for the staged fixture.
fn codes(name: &str) -> Vec<String> {
    let dir = stage(name);
    let args: Vec<String> = [
        "--cwd",
        dir.to_str().expect("utf8"),
        "--format",
        "json",
        "check",
    ]
    .iter()
    .map(|s| (*s).to_owned())
    .collect();
    let (mut out, mut err) = (Vec::new(), Vec::new());
    cadetta::run_with_env(&args, None, &mut out, &mut err);
    let _ = fs::remove_dir_all(&dir);
    let report: Value = serde_json::from_slice(&out).expect("json report");
    let mut found = Vec::new();
    collect(&report, &mut found);
    found
}

/// Gathers the `code` of every diagnostic object in `v`.
fn collect(v: &Value, out: &mut Vec<String>) {
    match v {
        Value::Object(m) => {
            if m.contains_key("severity")
                && let Some(c) = m.get("code").and_then(Value::as_str)
            {
                out.push(c.to_owned());
            }
            m.values().for_each(|x| collect(x, out));
        }
        Value::Array(a) => a.iter().for_each(|x| collect(x, out)),
        _ => {}
    }
}

// frob:tests crates/cadetta-cli/src/lint_facts.rs::interfaces
#[test]
fn interface_bound_violation_is_reported_through_check() {
    let c = codes("c001-bound-violated.must-fire");
    assert!(c.iter().any(|x| x == "C001"), "expected C001, got {c:?}");
}

// frob:tests crates/cadetta-cli/src/lint_facts.rs::interfaces
#[test]
fn interface_bound_satisfied_reports_no_c001() {
    let c = codes("c001-bound-violated.must-not-fire");
    assert!(!c.iter().any(|x| x == "C001"), "unexpected C001 in {c:?}");
}
