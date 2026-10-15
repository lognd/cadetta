//! frob:ticket 01M4YMHQ5BQW8T67PVT5ZFM9K6
//! The `ir-validate` and `lint` conformance entries (config-cli.md 3.9) over the ir corpus.

use std::path::{Path, PathBuf};

fn corpus() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/conformance/ir")
}

fn run(args: &[&str]) -> (u8, String) {
    let args: Vec<String> = args.iter().map(|s| (*s).to_owned()).collect();
    let mut out = Vec::new();
    let code = cadetta::run(&args, &mut out);
    (code, String::from_utf8(out).expect("utf8"))
}

/// Fixture names whose expect.json uses `entry`.
fn fixtures(entry: &str) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(corpus())
        .expect("corpus")
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .filter_map(|n| n.strip_suffix(".expect.json").map(str::to_owned))
        .filter(|n| {
            let text = std::fs::read_to_string(corpus().join(format!("{n}.expect.json"))).unwrap();
            text.contains(&format!("\"entry\": \"{entry}\""))
        })
        .collect();
    names.sort();
    names
}

// frob:tests crates/cadetta-cli/src/cmd/ir_validate.rs::run_ir_validate kind=unit
#[test]
fn ir_validate_corpus_matches_expected_exit_codes() {
    let names = fixtures("ir-validate");
    assert!(names.len() >= 50, "{}", names.len());
    let mut bad = Vec::new();
    for n in &names {
        for (side, want) in [("must-fire", 2), ("must-not-fire", 0)] {
            let file = corpus().join(format!("{n}.{side}.json"));
            let (code, out) = run(&["ir-validate", file.to_str().unwrap()]);
            if code != want || (want == 0 && !out.is_empty()) || (want == 2 && out.is_empty()) {
                bad.push(format!("{n}.{side}: exit {code}, output {out:?}"));
            }
        }
    }
    assert!(bad.is_empty(), "{bad:#?}");
}

fn fixture(name: &str) -> String {
    corpus().join(name).to_str().unwrap().to_owned()
}

// frob:tests crates/cadetta-cli/src/cmd/ir_lint.rs::run_lint_ir kind=unit
#[test]
fn lint_fires_d003_with_exit_1_and_stays_quiet_with_exit_0() {
    let (code, out) = run(&[
        "lint",
        &fixture("d003-ir.must-fire.json"),
        "--format",
        "json",
    ]);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("D003"), "{out}");
    let (code, out) = run(&[
        "lint",
        &fixture("d003-ir.must-not-fire.json"),
        "--format",
        "json",
    ]);
    assert_eq!(code, 0, "{out}");
    assert!(!out.contains("D003"), "{out}");
}

// frob:tests crates/cadetta-cli/src/cmd/ir_lint.rs::run_lint_ir kind=unit
#[test]
fn lint_of_an_invalid_document_exits_2_before_rules() {
    let (code, out) = run(&["lint", &fixture("modules-sorted.must-fire.json")]);
    assert_eq!(code, 2, "{out}");
    assert!(!out.contains("D003"), "{out}");
}

// frob:tests crates/cadetta-cli/src/cmd/ir_lint.rs::run_lint_ir kind=unit
#[test]
fn lint_text_output_names_the_rule() {
    let (code, out) = run(&["lint", &fixture("d003-ir.must-fire.json")]);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("D003"), "{out}");
}

// frob:tests crates/cadetta-cli/src/cmd/ir_validate.rs::run_ir_validate kind=unit
#[test]
fn ir_entries_reject_missing_unreadable_and_non_json_input_with_exit_2() {
    let dir = std::env::temp_dir().join(format!("cadetta-ir-entries-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let junk = dir.join("junk.json");
    std::fs::write(&junk, "not json").unwrap();
    for verb in ["ir-validate", "lint"] {
        assert_eq!(run(&[verb, junk.to_str().unwrap()]).0, 2, "{verb} junk");
        assert_eq!(
            run(&[verb, dir.join("absent.json").to_str().unwrap()]).0,
            2,
            "{verb} absent"
        );
        assert_eq!(run(&[verb]).0, 2, "{verb} usage");
    }
    assert_eq!(
        run(&["lint", junk.to_str().unwrap(), "--format", "xml"]).0,
        2
    );
    std::fs::remove_dir_all(&dir).unwrap();
}
