//! frob:ticket 01M4YTBH7QGW3Z87V77FY0KQ76
//! R014 through `cadetta sync`: the manual (no `drive`) emit fallback must raise R014 for a bound
//! Text parameter and withhold the file, like `cadetta emit` does.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

/// Copies a conformance corpus directory to a scratch dir.
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
    let to = std::env::temp_dir().join(format!("cadetta-r014sync-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&to);
    copy(&from, &to);
    to
}

/// Runs `cadetta --format json sync` in `dir`; returns (exit, report, written file text).
fn sync(dir: &Path, out_file: &str) -> (u8, Value, Option<String>) {
    let args: Vec<String> = [
        "--cwd",
        dir.to_str().expect("utf8"),
        "--format",
        "json",
        "sync",
    ]
    .iter()
    .map(|s| (*s).to_owned())
    .collect();
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let exit = cadetta::run_with_env(&args, None, &mut out, &mut err);
    let report = serde_json::from_slice(&out).expect("json report");
    let file = fs::read_to_string(dir.join(out_file)).ok();
    let _ = fs::remove_dir_all(dir);
    (exit, report, file)
}

const FIXTURE: &str = "emitters/sw-equations/r014-text-dimension-locator";

// frob:tests crates/cadetta-cli/src/cmd/sync.rs::manual_adapter kind=integration
#[test]
fn sync_manual_fallback_raises_r014_and_withholds_the_file() {
    let (_, report, file) = sync(
        &stage(&format!("{FIXTURE}.must-fire"), "fire"),
        "build/eq.txt",
    );
    assert!(file.is_none(), "file must be withheld: {report}");
    assert!(
        report["diagnostics"]
            .as_array()
            .expect("list")
            .iter()
            .any(|d| d["code"] == "R014"),
        "{report}"
    );
}

// frob:tests crates/cadetta-cli/src/cmd/sync.rs::manual_adapter kind=integration
#[test]
fn sync_manual_fallback_stays_clean_when_text_is_filtered_out() {
    let (_, report, file) = sync(
        &stage(
            &format!(
                "{}.must-not-fire",
                "emitters/sw-equations/r014-text-filtered-out"
            ),
            "ok",
        ),
        "build/eq.txt",
    );
    assert!(
        !report["diagnostics"]
            .as_array()
            .expect("list")
            .iter()
            .any(|d| d["code"] == "R014"),
        "{report}"
    );
    assert!(file.expect("written").contains("\"W\" = 10mm"));
}
