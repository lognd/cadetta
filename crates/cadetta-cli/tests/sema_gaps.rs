//! frob:ticket 01M4V13RVF8WWS2VMYE8WN9V3R
//! Sema gaps closed against the language conformance fixtures: E002 use targets,
//! E003 duplicate definitions as a diagnostic, E006 argument names.

use std::fs;
use std::path::PathBuf;

use cadetta::pipeline::analyze_at;
use cadetta::project::Project;

/// Error codes of `src` analysed as a one-package project's `src/lib.cdt`.
fn codes(tag: &str, src: &str) -> Vec<String> {
    let dir = std::env::temp_dir().join(format!("cadetta-sema-gaps-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("src")).expect("mkdir");
    fs::write(
        dir.join("cadetta.toml"),
        "[package]\nname = \"acme.demo\"\nversion = \"0.1.0\"\n",
    )
    .expect("config");
    fs::write(dir.join("src/lib.cdt"), src).expect("source");
    let (project, scope) = Project::open(Some(&dir), None).expect("project opens");
    let analysis = analyze_at(&project, &scope, "2026-10-14");
    let _ = fs::remove_dir_all(&dir);
    analysis
        .diagnostics
        .iter()
        .filter(|d| d.code.starts_with('E'))
        .map(|d| d.code.to_string())
        .collect()
}

/// The conformance fixture `name` with `side` (`must-fire` or `must-not-fire`).
fn fixture(name: &str, side: &str) -> String {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/conformance/language")
        .join(format!("{name}.{side}.cdt"));
    fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

/// Asserts the fixture fires exactly `code` once and its clean side fires no E code.
fn pair(name: &str, code: &str) {
    let fire = codes(&format!("{name}-f"), &fixture(name, "must-fire"));
    assert_eq!(fire, vec![code.to_owned()], "{name} must-fire");
    let clean = codes(&format!("{name}-c"), &fixture(name, "must-not-fire"));
    assert!(clean.is_empty(), "{name} must-not-fire: {clean:?}");
}

// frob:tests crates/cadetta-sema/src/resolve.rs::check_use_targets kind=integration
#[test]
fn e002_use_target_unknown_package() {
    pair("e002-use-target", "E002");
}

// frob:tests crates/cadetta-sema/src/resolve.rs::check_use_targets kind=integration
#[test]
fn e002_use_target_unknown_item_in_known_module() {
    let c = codes("e002-item", "use self::Missing\nmodel P {\n  w = 3mm\n}\n");
    assert_eq!(c, vec!["E002"]);
}

// frob:tests crates/cadetta-cli/src/pipeline.rs::analyze_at kind=integration
#[test]
fn e003_duplicate_type_is_a_diagnostic() {
    pair("e003-duplicate-type", "E003");
}

// frob:tests crates/cadetta-cli/src/pipeline.rs::analyze_at kind=integration
#[test]
fn e006_argument_name_matches_no_parameter() {
    pair("e006-argument-mismatch", "E006");
}

// frob:tests crates/cadetta-cli/src/pipeline.rs::analyze_at kind=integration
#[test]
fn e006_too_many_positional_arguments() {
    let c = codes(
        "e006-pos",
        "model Axle(span: Length) {\n  length = span\n}\nassembly Rig {\n  part a: Axle(3mm, 4mm)\n}\n",
    );
    assert_eq!(c, vec!["E006"]);
}
