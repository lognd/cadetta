//! frob:ticket 01M4V7WCM9TBRWY007WWS81S98
//! R015: a dimension/property/frame locator or `ref` binding the run must emit to a json or csv
//! file is an error; one that is filtered out or unbound is silent (emitters/json.md 1).

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

/// Copies `fixture` (a corpus directory, relative to tests/conformance) to a scratch dir.
fn stage(fixture: &str, tag: &str) -> PathBuf {
    fn copy(from: &Path, to: &Path) {
        fs::create_dir_all(to).expect("mkdir");
        for e in fs::read_dir(from).expect("read_dir").flatten() {
            let dest = to.join(e.file_name());
            if e.file_type().expect("type").is_dir() {
                copy(&e.path(), &dest);
            } else {
                fs::copy(e.path(), &dest).expect("copy");
            }
        }
    }
    let from = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/conformance")
        .join(fixture);
    let to = std::env::temp_dir().join(format!("cadetta-r015-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&to);
    copy(&from, &to);
    to
}

/// Runs `cadetta --format json emit` in `dir`; returns (exit, report).
fn emit(dir: &Path) -> (u8, Value) {
    let args: Vec<String> = [
        "--cwd",
        dir.to_str().expect("utf8"),
        "--format",
        "json",
        "emit",
    ]
    .iter()
    .map(|s| (*s).to_owned())
    .collect();
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let exit = cadetta::run_with_env(&args, None, &mut out, &mut err);
    let report = serde_json::from_slice(&out).expect("json report");
    let _ = fs::remove_dir_all(dir);
    (exit, report)
}

const NAMES: [&str; 8] = [
    "emitters/json/r015-dimension-locator",
    "emitters/json/r015-property-locator",
    "emitters/json/r015-ref-mode",
    "emitters/json/r015-filtered-out",
    "emitters/csv/r015-dimension-locator",
    "emitters/csv/r015-property-locator",
    "emitters/csv/r015-ref-mode",
    "emitters/csv/r015-filtered-out",
];

// frob:tests crates/cadetta-cli/src/cmd/emit.rs::value_row_diagnostics kind=integration
#[test]
fn unsupported_locator_on_json_or_csv_is_r015_and_exit_1() {
    for fixture in NAMES {
        let (exit, report) = emit(&stage(&format!("{fixture}.must-fire"), "fire"));
        assert_eq!(exit, 1, "{fixture}: {report}");
        let d = &report["diagnostics"][0];
        assert_eq!(d["code"], "R015", "{fixture}: {report}");
        assert_eq!(d["severity"], "error", "{fixture}");
        assert!(d["path"].as_str().is_some(), "{fixture}: path named");
        assert!(
            d["message"].as_str().expect("msg").contains("locator"),
            "{fixture}"
        );
        assert!(
            report["files"].as_array().is_none_or(Vec::is_empty),
            "{fixture}: withheld"
        );
    }
}

// frob:tests crates/cadetta-cli/src/cmd/emit.rs::value_row_diagnostics kind=integration
#[test]
fn filtered_or_unbound_locator_is_not_r015() {
    for fixture in NAMES {
        let (_, report) = emit(&stage(&format!("{fixture}.must-not-fire"), "ok"));
        let diags = report["diagnostics"].as_array().expect("list");
        assert!(
            diags.iter().all(|d| d["code"] != "R015"),
            "{fixture}: {report}"
        );
    }
}
