//! frob:ticket 01M4V2J34YBSMM32SK26K58FR1
//! A wrong kind on an implementer's interface member: dimension mismatch is I001, a named-kind
//! mismatch is Q003 (stdlib.md 6, language.md 3.5).

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

/// Stages the stdlib fixture `name` (e.g. `i001-sourced-wrong-kind.must-fire`) as a package.
fn stage(name: &str) -> PathBuf {
    let src = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/conformance/stdlib")
        .join(format!("{name}.cdt"));
    let dir = std::env::temp_dir().join(format!("cadetta-sik-{name}-{}", std::process::id()));
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

/// Error-severity codes `cadetta check` reports for the staged fixture.
fn error_codes(name: &str) -> Vec<String> {
    let dir = stage(name);
    let args: Vec<String> = ["--cwd", dir.to_str().expect("utf8"), "--format", "json", "check"]
        .iter()
        .map(|s| (*s).to_owned())
        .collect();
    let (mut out, mut err) = (Vec::new(), Vec::new());
    cadetta::run_with_env(&args, None, &mut out, &mut err);
    let _ = fs::remove_dir_all(&dir);
    let report: Value = serde_json::from_slice(&out).expect("json report");
    let mut codes = Vec::new();
    collect(&report, &mut codes);
    codes
}

fn collect(v: &Value, out: &mut Vec<String>) {
    match v {
        Value::Object(m) => {
            if m.get("severity").and_then(Value::as_str) == Some("error")
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

// frob:tests crates/cadetta-sema/src/kindcheck.rs::check_interface_members kind=integration
#[test]
fn wrong_dimension_on_sourced_member_is_i001() {
    assert_eq!(error_codes("i001-sourced-wrong-kind.must-fire"), ["I001"]);
    assert!(error_codes("i001-sourced-wrong-kind.must-not-fire").is_empty());
}

// frob:tests crates/cadetta-sema/src/kindcheck.rs::check_interface_members kind=integration
#[test]
fn named_kind_mismatch_on_sourced_member_is_q003() {
    assert_eq!(error_codes("q003-sourced-named-kind.must-fire"), ["Q003"]);
    assert!(error_codes("q003-sourced-named-kind.must-not-fire").is_empty());
}

// frob:tests crates/cadetta-sema/src/kindcheck.rs::check_interface_members kind=integration
#[test]
fn optional_sourced_members_may_be_absent() {
    assert_eq!(error_codes("i001-sourced-optional-members.must-fire"), ["I001"]);
    assert!(error_codes("i001-sourced-optional-members.must-not-fire").is_empty());
}
