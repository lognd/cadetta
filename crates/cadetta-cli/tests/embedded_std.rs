//! frob:ticket 01M4TA3CCFZS5X2P2YESK4E68X
//! `std` resolves from the copy embedded in the binary, with no `std/` directory on disk.

use std::fs;
use std::path::PathBuf;

use cadetta::pipeline::{Analysis, analyze};
use cadetta::project::Project;

/// Writes a one-package project with `src` as `src/lib.cdt` and analyzes it.
fn analyze_src(tag: &str, src: &str) -> Analysis {
    let dir: PathBuf =
        std::env::temp_dir().join(format!("cadetta-embedded-std-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("src")).expect("mkdir");
    fs::write(
        dir.join("cadetta.toml"),
        "[package]\nname = \"acme.demo\"\nversion = \"0.1.0\"\n",
    )
    .expect("config");
    fs::write(dir.join("src/lib.cdt"), src).expect("source");
    let (project, scope) = Project::open(Some(&dir), None).expect("project opens");
    let analysis = analyze(&project, &scope);
    let _ = fs::remove_dir_all(&dir);
    analysis
}

fn errors(a: &Analysis) -> Vec<String> {
    a.diagnostics
        .iter()
        .filter(|d| format!("{:?}", d.severity).contains("Error"))
        .map(|d| format!("{} {}", d.code, d.message))
        .collect()
}

// frob:tests crates/cadetta-cli/src/pipeline.rs::analyze kind=integration
#[test]
fn std_import_resolves_from_the_embedded_copy() {
    assert!(
        std::env::var_os("CADETTA_STD").is_none(),
        "test needs CADETTA_STD unset"
    );
    let good = analyze_src(
        "good",
        "use std::materials::Material\n\nmodel Part {\n  material = Material::Al6061T6\n}\n",
    );
    assert!(errors(&good).is_empty(), "errors: {:?}", errors(&good));
    assert!(
        good.sources.files().iter().any(|f| f.is_std),
        "std modules were indexed"
    );
    // Negative control: a variant std does not define still errors, so the pass above is real.
    let bad = analyze_src(
        "bad",
        "use std::materials::Material\n\nmodel Part {\n  material = Material::NoSuchVariant\n}\n",
    );
    assert!(!errors(&bad).is_empty(), "unknown std item must error");
}
