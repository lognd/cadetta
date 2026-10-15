//! frob:ticket 01M4V7WC7QPQ4RH2902T8PTE0A
//! R014: a bound Text parameter the run must emit is an error and withholds the file; one that is
//! filtered out or unbound is silently not emitted (emitters/sw-equations.md 1).

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
    let to = std::env::temp_dir().join(format!("cadetta-r014-{tag}-{}", std::process::id()));
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

const NAMES: [&str; 3] = [
    "emitters/sw-equations/r014-text-dimension-locator",
    "emitters/sw-equations/r014-text-filtered-out",
    "emitters/sw-equations/value-text-not-emitted",
];

// frob:tests crates/cadetta-cli/src/cmd/emit.rs::text_diagnostics kind=integration
#[test]
fn bound_text_parameter_is_r014_and_withholds_the_file() {
    for fixture in NAMES {
        let (exit, report, file) = emit(
            &stage(&format!("{fixture}.must-fire"), "fire"),
            "build/eq.txt",
        );
        assert_eq!(exit, 1, "{fixture}: {report}");
        assert!(file.is_none(), "{fixture}: file must be withheld");
        let d = &report["diagnostics"][0];
        assert_eq!(d["code"], "R014", "{fixture}: {report}");
        assert_eq!(d["severity"], "error", "{fixture}");
        assert_eq!(d["path"], "acme.demo::Part.note", "{fixture}");
        assert!(
            d["message"].as_str().expect("msg").contains("Text"),
            "{fixture}"
        );
    }
}

// frob:tests crates/cadetta-cli/src/cmd/emit.rs::text_diagnostics kind=integration
#[test]
fn filtered_or_unbound_text_parameter_is_not_emitted_and_not_an_error() {
    for fixture in NAMES {
        let (exit, report, file) = emit(
            &stage(&format!("{fixture}.must-not-fire"), "ok"),
            "build/eq.txt",
        );
        assert_eq!(exit, 0, "{fixture}: {report}");
        assert!(
            report["diagnostics"].as_array().expect("list").is_empty(),
            "{fixture}"
        );
        let text = file.expect("written");
        assert!(text.contains("\"W\" = 10mm"), "{fixture}: {text}");
        assert!(
            !text.contains("Note") && !text.contains("anodize"),
            "{fixture}: {text}"
        );
    }
}
