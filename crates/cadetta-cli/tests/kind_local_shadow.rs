//! frob:ticket 01M4Y6TG02JC7RX0PBN8HV1QQY
//! A block `let` of unknown type shadows a same-named component member in the kind checker
//! (quantities.md 4.4), so the member's dimension is not read in its place.

use std::fs;

use cadetta::pipeline::{Analysis, analyze_at};
use cadetta::project::Project;

/// Writes a package with the given `src`-relative files and analyzes it.
fn analyze_files(tag: &str, files: &[(&str, &str)]) -> Analysis {
    let dir = std::env::temp_dir().join(format!("cadetta-shadow-{tag}-{}", std::process::id()));
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

/// Codes of a one-module package whose `Axle.seat: Mass` is the block `body`; `width: Length`
/// is a sibling member.
fn seat_codes(tag: &str, body: &str) -> Vec<String> {
    let src = format!("model Axle {{\n  width: Length = 4mm\n  seat: Mass = {body}\n}}\n");
    codes(&analyze_files(tag, &[("lib.cdt", &src)]))
}

// frob:tests crates/cadetta-sema/src/kindcheck.rs::KindChecker kind=integration
#[test]
fn untyped_block_local_shadows_same_named_member() {
    let c = seat_codes("shadow", "{ let width = \"w\"; width }");
    assert!(!c.iter().any(|x| x.starts_with('Q')), "{c:?}");
}

// frob:tests crates/cadetta-sema/src/kindcheck.rs::KindChecker kind=integration
#[test]
fn member_read_and_typed_local_keep_their_behaviour() {
    let member = seat_codes("member", "width");
    assert!(member.iter().any(|x| x == "Q002"), "{member:?}");
    let typed = seat_codes("typed", "{ let width = 2mm; width }");
    assert!(typed.iter().any(|x| x == "Q002"), "{typed:?}");
}
