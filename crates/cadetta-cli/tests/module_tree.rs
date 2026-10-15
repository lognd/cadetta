//! frob:ticket 01M4V6BZ4W0RZ3AVHKQTHJT2H7
//! Module-tree diagnostics: a bad file name is E015 (not E001) and nested
//! snake_case modules do not trip N001 (packages.md 2, 7).

use std::fs;

use cadetta::pipeline::{analyze_at, Analysis};
use cadetta::project::Project;

/// Writes a package with the given `src`-relative files and analyzes it.
fn analyze_files(tag: &str, files: &[(&str, &str)]) -> Analysis {
    let dir = std::env::temp_dir().join(format!("cadetta-modtree-{tag}-{}", std::process::id()));
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

// frob:tests crates/cadetta-sema/src/modules.rs::module_path_of kind=integration
#[test]
fn bad_module_file_name_is_e015_not_e001() {
    let a = analyze_files("bad", &[("lib.cdt", ""), ("BadName.cdt", "")]);
    let c = codes(&a);
    assert!(c.iter().any(|x| x == "E015"), "{c:?}");
    assert!(!c.iter().any(|x| x == "E001"), "{c:?}");
}

// frob:tests crates/cadetta-lint/src/rules/n.rs::check kind=integration
#[test]
fn nested_snake_case_module_has_no_n001() {
    let a = analyze_files(
        "nested",
        &[("lib.cdt", ""), ("drive/motor.cdt", "model Part {}\n")],
    );
    let c = codes(&a);
    assert!(!c.iter().any(|x| x == "N001"), "{c:?}");
}
