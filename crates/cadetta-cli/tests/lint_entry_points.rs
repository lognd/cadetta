//! frob:ticket 01M4TA3CABXHEZFA7BXA8J3XJW
//! I001, I004, U002, `require` protection and `#[allow]` fire through `analyze` (~A8J3XJW).

use std::fs;
use std::path::PathBuf;

use cadetta::pipeline::{Analysis, analyze};
use cadetta::project::Project;
use cadetta_lint::engine::{LintConfig, apply};

/// Writes a one-package project with `src` as `src/lib.cdt` and analyzes it.
fn analyze_src(tag: &str, src: &str) -> Analysis {
    let dir: PathBuf =
        std::env::temp_dir().join(format!("cadetta-lint-entry-{tag}-{}", std::process::id()));
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

fn codes(a: &Analysis) -> Vec<(String, Option<String>)> {
    a.diagnostics
        .iter()
        .map(|d| (d.code.clone(), d.path.clone()))
        .collect()
}

fn has(a: &Analysis, code: &str, needle: &str) -> bool {
    a.diagnostics
        .iter()
        .any(|d| d.code == code && d.path.as_deref().is_some_and(|p| p.contains(needle)))
}

// frob:tests crates/cadetta-lint/src/rules/interface.rs::check_members kind=unit
#[test]
fn i001_fires_when_an_implementer_lacks_an_interface_member() {
    let a = analyze_src(
        "i001",
        "pub interface Shaft {\n  length: Length\n  datum axis\n}\n\npub model Bad: Shaft {\n  width = 1mm\n}\n",
    );
    assert!(!a.halted);
    assert!(has(&a, "I001", "Bad.length"), "{:?}", codes(&a));
    assert!(has(&a, "I001", "Bad.axis"), "{:?}", codes(&a));
}

// frob:tests crates/cadetta-lint/src/rules/interface.rs::check_members kind=unit
#[test]
fn i001_is_quiet_when_the_implementer_has_every_member() {
    let a = analyze_src(
        "i001ok",
        "pub interface Shaft {\n  length: Length\n}\n\npub model Good: Shaft {\n  length = 5mm\n}\n",
    );
    assert!(
        !a.diagnostics.iter().any(|d| d.code == "I001"),
        "{:?}",
        codes(&a)
    );
}

// frob:tests crates/cadetta-lint/src/rules/interface.rs::check_roots kind=unit
#[test]
fn i004_fires_for_a_public_parameterized_root() {
    let a = analyze_src(
        "i004",
        "pub model Root(width: Length) {\n  area = width * width\n}\n",
    );
    assert!(has(&a, "I004", "Root"), "{:?}", codes(&a));
}

// frob:tests crates/cadetta-lint/src/rules/u002.rs::check kind=unit
#[test]
fn u002_fires_for_an_unused_private_item_and_not_for_a_used_one() {
    let a = analyze_src(
        "u002",
        "fn dead(x: Length) -> Length { x }\n\nfn live(x: Length) -> Length { x }\n\npub model M {\n  a = live(2mm)\n}\n",
    );
    assert!(has(&a, "U002", "dead"), "{:?}", codes(&a));
    assert!(!has(&a, "U002", "live"), "{:?}", codes(&a));
}

// frob:ticket 01M4YEA3SADYRF5MJ4AG2DFAFQ
// frob:tests crates/cadetta-cli/src/lint_facts.rs::item_facts kind=unit
#[test]
fn u002_use_is_by_resolution_so_a_shadowing_name_is_not_a_use() {
    // `dead` is mentioned twice more, but only as `g`'s parameter, which shadows it.
    let a = analyze_src(
        "u002-shadow",
        "fn dead(x: Length) -> Length { x }\n\nfn g(dead: Length) -> Length { dead }\n\npub model M {\n  a = g(2mm)\n}\n",
    );
    assert!(has(&a, "U002", "::dead"), "{:?}", codes(&a));
    assert!(!has(&a, "U002", "::g"), "{:?}", codes(&a));
}

// frob:ticket 01M4YEA3SADYRF5MJ4AG2DFAFQ
// frob:tests crates/cadetta-cli/src/lint_facts.rs::item_facts kind=unit
#[test]
fn u002_reports_an_unused_private_member_and_not_a_read_one() {
    let a = analyze_src(
        "u002-member",
        "pub model M {\n  priv spare = 1mm\n  priv used = 2mm\n  a = used\n}\n",
    );
    assert!(has(&a, "U002", "M.spare"), "{:?}", codes(&a));
    assert!(!has(&a, "U002", "M.used"), "{:?}", codes(&a));
}

// frob:ticket 01M4YEA3SADYRF5MJ4AG2DFAFQ
// frob:tests crates/cadetta-cli/src/lint_facts.rs::item_facts kind=unit
#[test]
fn i004_needs_a_parameter_without_default_and_no_instantiating_part() {
    let a = analyze_src(
        "i004-facts",
        "pub model Leaf(w: Length) {\n  area = w * w\n}\n\npub model Defaulted(w: Length = 1mm) {\n  area = w * w\n}\n\npub model Top {\n  part l: Leaf(w = 1mm)\n}\n",
    );
    assert!(!has(&a, "I004", "Leaf"), "instantiated: {:?}", codes(&a));
    assert!(
        !has(&a, "I004", "Defaulted"),
        "all defaulted: {:?}",
        codes(&a)
    );
}

// frob:tests crates/cadetta-lint/src/engine.rs::apply kind=unit
#[test]
fn allow_attribute_suppresses_through_the_engine() {
    let src = "pub model M {\n  #[allow(M001, \"vendor drawing 1234\")]\n  w = 12.3456mm\n}\n";
    let a = analyze_src("allow", src);
    assert!(
        a.diagnostics.iter().any(|d| d.code == "M001"),
        "M001 must fire first: {:?}",
        codes(&a)
    );
    assert!(!a.suppressions.allows.is_empty(), "allow not collected");
    let out = apply(a.diagnostics.clone(), &LintConfig::new(), &a.suppressions);
    let m001 = out
        .diagnostics
        .iter()
        .find(|d| d.code == "M001")
        .expect("M001");
    assert!(m001.waived.is_some(), "allow did not suppress M001");
}

// frob:tests crates/cadetta-lint/src/engine.rs::apply kind=unit
#[test]
fn require_paths_are_collected_so_requires_are_never_suppressed() {
    let a = analyze_src(
        "require",
        "pub model M {\n  w = 5mm\n  require wide: w > 1mm\n}\n",
    );
    assert!(
        a.suppressions
            .require_paths
            .iter()
            .any(|p| p.ends_with("M.wide")),
        "{:?}",
        a.suppressions.require_paths
    );
}
