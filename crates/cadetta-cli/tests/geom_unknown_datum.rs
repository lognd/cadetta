//! frob:ticket 01M4Z0WGJPRDATAN7V7N8B2SV9
//! An unknown datum of a placed part on the right of `in` is E002, not EVAL (geometry.md 3).

use std::fs;
use std::path::PathBuf;

use cadetta::pipeline::analyze_at;
use cadetta::project::Project;

/// Diagnostic codes of the geometry fixture `name`.`side` analysed as a one-package project.
fn codes(name: &str, side: &str) -> Vec<String> {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/conformance/geometry")
        .join(format!("{name}.{side}.cdt"));
    let src = fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
    let dir =
        std::env::temp_dir().join(format!("cadetta-geom-datum-{side}-{}", std::process::id()));
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

// frob:tests crates/cadetta-sema/src/geom.rs::reexpress kind=integration
#[test]
fn unknown_datum_of_placed_part_is_e002() {
    assert_eq!(
        codes("datum-unknown-of-placed-part", "must-fire"),
        vec!["E002"]
    );
    assert!(codes("datum-unknown-of-placed-part", "must-not-fire").is_empty());
}
