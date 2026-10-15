//! frob:ticket 01M4YQFTG7GPRHE6E1PH29EAMF
//! Q004: a value not representable at the adapter's declared precision is an error, never
//! rounded, and withholds the file (quantities.md 1, adapters.md 4, emitters/sw-equations.md 1).

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
    let to = std::env::temp_dir().join(format!("cadetta-q004-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&to);
    copy(&from, &to);
    to
}

/// Runs `cadetta --format json emit` in `dir`; returns (exit, report, written file text).
fn emit(dir: &Path, out_file: &str) -> (u8, Value, Option<String>) {
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
    let file = fs::read_to_string(dir.join(out_file)).ok();
    let _ = fs::remove_dir_all(dir);
    (exit, report, file)
}

// frob:tests crates/cadetta-cli/src/cmd/emit.rs::diag kind=integration
#[test]
fn q004_beyond_precision_fires_and_withholds_the_file() {
    for (fixture, path) in [
        (
            "emitters/sw-equations/value-q004-beyond-precision",
            "acme.demo::Part.width",
        ),
        (
            "adapters/contract-q004-not-representable",
            "acme.demo::Part.third",
        ),
        (
            "adapters/contract-structured-failures",
            "acme.demo::Part.third",
        ),
    ] {
        let (exit, report, file) = emit(
            &stage(&format!("{fixture}.must-fire"), "fire"),
            "build/eq.txt",
        );
        assert_eq!(exit, 1, "{fixture}");
        assert!(file.is_none(), "{fixture}: file must be withheld");
        let d = &report["diagnostics"][0];
        assert_eq!(d["code"], "Q004", "{fixture}: {report}");
        assert_eq!(d["severity"], "error", "{fixture}");
        assert_eq!(d["schema"], "cadetta.diag/1", "{fixture}");
        assert_eq!(d["path"], path, "{fixture}");
    }
}

// frob:tests crates/cadetta-cli/src/cmd/emit.rs::diag kind=integration
#[test]
fn representable_value_is_written_at_the_declared_precision() {
    let (exit, report, file) = emit(
        &stage(
            "emitters/sw-equations/value-q004-beyond-precision.must-not-fire",
            "ok",
        ),
        "build/eq.txt",
    );
    assert_eq!(exit, 0, "{report}");
    assert!(report["diagnostics"].as_array().expect("list").is_empty());
    assert!(file.expect("written").contains("\"W\" = 0.12mm"));
}

// frob:tests crates/cadetta-cli/src/cmd/emit.rs::diag kind=integration
#[test]
fn quantity_q004_fixture_fires_for_one_third_at_two_decimals() {
    let (exit, report, _) = emit(&stage("quantities/q004.must-fire", "qty"), "build/none");
    assert_eq!(exit, 1, "{report}");
    assert_eq!(report["diagnostics"][0]["path"], "acme.demo::P.w");
    let (exit, _, _) = emit(
        &stage("quantities/q004.must-not-fire", "qty-ok"),
        "build/none",
    );
    assert_eq!(exit, 0);
}
