//! frob:ticket 01M4YR00X3TTZGSBAKQZDNTT6T
//! Privacy diagnostics wired into the pipeline: E016 on `use` targets, E017 on signatures.

use std::path::PathBuf;

use cadetta::pipeline::analyze_at;
use cadetta::project::Project;

/// Error codes of the language conformance fixture directory `name` with `side`.
fn codes(name: &str, side: &str) -> Vec<String> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/conformance/language")
        .join(format!("{name}.{side}"));
    let (project, scope) = Project::open(Some(&dir), None).expect("project opens");
    analyze_at(&project, &scope, "2026-10-14")
        .diagnostics
        .iter()
        .filter(|d| d.code.starts_with('E'))
        .map(|d| d.code.to_string())
        .collect()
}

// frob:tests crates/cadetta-sema/src/visibility.rs::check_uses kind=integration
#[test]
fn e016_private_item_used_from_sibling_module() {
    assert_eq!(codes("e016-private-item", "must-fire"), vec!["E016"]);
}

// frob:tests crates/cadetta-sema/src/visibility.rs::check_uses kind=integration
#[test]
fn e016_pub_item_used_from_sibling_module_is_clean() {
    assert!(codes("e016-private-item", "must-not-fire").is_empty());
}

// frob:tests crates/cadetta-sema/src/visibility.rs::check_signatures kind=integration
#[test]
fn e017_private_type_in_public_signature() {
    assert_eq!(codes("e017-private-in-public", "must-fire"), vec!["E017"]);
}

// frob:tests crates/cadetta-sema/src/visibility.rs::check_signatures kind=integration
#[test]
fn e017_public_type_in_public_signature_is_clean() {
    assert!(codes("e017-private-in-public", "must-not-fire").is_empty());
}
