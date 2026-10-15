//! frob:ticket 01M4V1HDYKBWX1HP3ZVPDC7173
//! C004 (derived feature in a check) reaches `check`, and E010 honours `#[role(..)]`
//! (~PDC7173, ~FSJCX9G; quantities.md 5.3 and 5.6).

use std::fs;

use cadetta::pipeline::{Analysis, analyze_at};
use cadetta::project::Project;

/// Writes a one-package project with `src` as `src/lib.cdt` and analyzes it.
fn analyze_src(tag: &str, src: &str) -> Analysis {
    let dir = std::env::temp_dir().join(format!("cadetta-c004-{tag}-{}", std::process::id()));
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

/// (code, path) of every diagnostic of `src` with the given code.
fn hits(tag: &str, src: &str, code: &str) -> Vec<Option<String>> {
    analyze_src(tag, src)
        .diagnostics
        .iter()
        .filter(|d| d.code == code)
        .map(|d| d.path.clone())
        .collect()
}

const DERIVED: &str =
    "model P {\n  head = 8mm [+-0.1mm]\n  cb = head + 1mm\n  check c: cb >= 9mm\n}\n";

// frob:tests crates/cadetta-lint/src/rules/c.rs::check_derived kind=integration
#[test]
fn c004_fires_on_derived_parameter_read_by_a_check() {
    assert_eq!(
        hits("fire", DERIVED, "C004"),
        vec![Some("acme.demo::P.cb".to_owned())]
    );
}

// frob:tests crates/cadetta-lint/src/rules/c.rs::check_derived kind=integration
#[test]
fn c004_quiet_for_let_and_unread_derived() {
    let with_let = DERIVED.replace("  cb =", "  let cb =");
    assert!(hits("let", &with_let, "C004").is_empty());
    let unread = DERIVED.replace("cb >= 9mm", "head >= 9mm");
    assert!(hits("unread", &unread, "C004").is_empty());
}

// frob:tests crates/cadetta-lint/src/rules/c.rs::check_derived kind=integration
#[test]
fn c004_fires_once_per_parameter_across_checks() {
    let two = format!("{}", DERIVED.replace("}\n", "  check d: cb <= 10mm\n}\n"));
    assert_eq!(hits("twice", &two, "C004").len(), 1);
}

// frob:tests crates/cadetta-cli/src/pipeline.rs::analyze_at kind=integration
#[test]
fn e010_honours_role_attribute_for_mmc_and_lmc() {
    let bare = "model Part {\n  w = 5mm [+-0.1mm]\n  x = w.mmc\n}\n";
    assert_eq!(hits("bare", bare, "E010").len(), 1);
    for role in ["hole", "shaft"] {
        for acc in ["mmc", "lmc"] {
            let src = format!(
                "model Part {{\n  #[role({role})]\n  w = 5mm [+-0.1mm]\n  x = w.{acc}\n}}\n"
            );
            assert!(
                hits(&format!("{role}{acc}"), &src, "E010").is_empty(),
                "{role} {acc}"
            );
        }
    }
}
