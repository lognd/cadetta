//! frob:ticket 01M4YMKH7NQW964KS1C2GJX813
//! User-declared `kind` names are registered, so Q003 fires through `analyze` (~2GJX813).

use std::fs;

use cadetta::pipeline::{analyze_at, Analysis};
use cadetta::project::Project;

/// Writes a one-package project with `src` as `src/lib.cdt` and analyzes it.
fn analyze_src(tag: &str, src: &str) -> Analysis {
    let dir = std::env::temp_dir().join(format!("cadetta-user-kinds-{tag}-{}", std::process::id()));
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
}

#[test]
fn user_declared_kind_fires_q003_on_anonymous_length() {
    let a = analyze_src(
        "fire",
        "kind Sink = Length\n\nmodel P {\n  s: Sink = cbrt(8mm^3)\n}\n",
    );
    assert!(
        a.diagnostics.iter().any(|d| d.code == "Q003"),
        "{:?}",
        a.diagnostics
    );
}

#[test]
fn user_declared_kind_quiet_when_target_is_length() {
    let a = analyze_src(
        "quiet",
        "kind Sink = Length\n\nmodel P {\n  s: Length = cbrt(8mm^3)\n}\n",
    );
    assert!(
        !a.diagnostics.iter().any(|d| d.code == "Q003"),
        "{:?}",
        a.diagnostics
    );
}
