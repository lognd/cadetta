//! frob:ticket 01M4Y6TC5EN0M1WMP9AK3YMAS8
//! `::` qualified member reads (`package::module::Comp.member`) carry their real dimension
//! through `EvalEnv::path_type` into the kind checker (quantities.md 4.4).

use std::fs;

use cadetta::pipeline::{Analysis, analyze_at};
use cadetta::project::Project;

/// Writes a package with the given `src`-relative files and analyzes it.
fn analyze_files(tag: &str, files: &[(&str, &str)]) -> Analysis {
    let dir = std::env::temp_dir().join(format!("cadetta-qpath-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("src")).expect("mkdir");
    fs::write(
        dir.join("cadetta.toml"),
        "[package]\nname = \"acme.demo\"\nversion = \"0.1.0\"\n",
    )
    .expect("config");
    for (rel, text) in files {
        let p = dir.join("src").join(rel);
        fs::create_dir_all(p.parent().expect("parent")).expect("mkdir");
        fs::write(p, text).expect("source");
    }
    let (project, scope) = Project::open(Some(&dir), None).expect("project opens");
    let analysis = analyze_at(&project, &scope, "2026-10-14");
    let _ = fs::remove_dir_all(&dir);
    analysis
}

/// Codes of every diagnostic, in order.
fn codes(a: &Analysis) -> Vec<String> {
    a.diagnostics.iter().map(|d| d.code.to_string()).collect()
}

/// A package with `drive::motor::Part.width: Length` and an `Axle` reading it as `seat: <kind>`.
fn read_width_as(tag: &str, kind: &str) -> Vec<String> {
    let lib = format!("model Axle {{\n  seat: {kind} = package::drive::motor::Part.width\n}}\n");
    let a = analyze_files(
        tag,
        &[
            ("lib.cdt", &lib),
            (
                "drive/motor.cdt",
                "pub model Part {\n  width: Length = 4mm\n}\n",
            ),
        ],
    );
    codes(&a)
}

// frob:tests crates/cadetta-sema/src/kindcheck.rs::EvalEnv kind=integration
#[test]
fn qualified_path_read_with_wrong_dimension_is_q002() {
    let c = read_width_as("wrong", "Mass");
    assert!(c.iter().any(|x| x == "Q002"), "{c:?}");
}

// frob:tests crates/cadetta-sema/src/kindcheck.rs::EvalEnv kind=integration
#[test]
fn qualified_path_read_with_matching_dimension_is_clean() {
    let c = read_width_as("match", "Length");
    let bad: Vec<_> = c
        .iter()
        .filter(|x| x.starts_with('Q') || x.starts_with('E'))
        .collect();
    assert!(bad.is_empty(), "{c:?}");
}
