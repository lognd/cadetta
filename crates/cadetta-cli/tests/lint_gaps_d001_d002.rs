//! frob:ticket 01M4V2KDSCB0QK5JHYG5KNEDZF
//! Regression cover for the lint gaps probed in ~5KNEDZF: D001 severity below
//! `d001_min_digits`, D002 against a fit accessor and the geometry pass reaching `check`
//! (lints.md D, geometry.md 7).

use std::fs;

use cadetta::pipeline::{Analysis, analyze_at};
use cadetta::project::Project;

/// Writes a one-package project with `src` as `src/lib.cdt` and analyzes it.
fn analyze_src(tag: &str, src: &str) -> Analysis {
    let dir = std::env::temp_dir().join(format!("cadetta-gaps-{tag}-{}", std::process::id()));
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

/// (severity text, path) of every diagnostic of `src` with the given code.
fn hits(tag: &str, src: &str, code: &str) -> Vec<(String, Option<String>)> {
    analyze_src(tag, src)
        .diagnostics
        .iter()
        .filter(|d| d.code == code)
        .map(|d| (format!("{:?}", d.severity), d.path.clone()))
        .collect()
}

// frob:tests crates/cadetta-lint/src/rules/d.rs::check_d001 kind=integration
#[test]
fn d001_is_info_below_min_digits_and_warning_at_or_above() {
    let low = "pub model P {\n  a = 5mm\n  b = 5mm\n}\n";
    let got = hits("d001-low", low, "D001");
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].0, "Info");
    let high = "pub model P {\n  a = 6.35mm\n  b = 6.35mm\n}\n";
    let got = hits("d001-high", high, "D001");
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].0, "Warning");
}

// frob:tests crates/cadetta-lint/src/rules/d.rs::check_d002 kind=integration
#[test]
fn d002_fires_for_a_literal_equal_to_a_fit_clearance_accessor() {
    let src = "pub model P {\n  bore = 20mm\n  fit seat = H7/p6 on bore\n  y = -0.035mm\n}\n";
    let got = hits("d002", src, "D002");
    assert_eq!(
        got,
        vec![("Warning".to_owned(), Some("acme.demo::P.y".to_owned()))]
    );
}

// frob:tests crates/cadetta-lint/src/rules/d.rs::check_d002 kind=integration
#[test]
fn d002_quiet_for_a_literal_unlike_any_fit_accessor() {
    let src = "pub model P {\n  bore = 20mm\n  fit seat = H7/p6 on bore\n  y = -0.036mm\n}\n";
    assert!(hits("d002-quiet", src, "D002").is_empty());
}

// frob:ticket 01M51NC9FMMRMKC3NVMJKF14FE
// frob:tests crates/cadetta-lint/src/rules/d.rs::check_d002 kind=integration
#[test]
fn d002_quiet_for_the_authored_source_of_a_fit_side() {
    let src = "pub model P {\n  bore = 20mm\n  fit seat = H7/p6 on bore\n}\n";
    assert!(hits("d002-source", src, "D002").is_empty());
}

/// The source of a conformance geometry fixture.
fn fixture(name: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/conformance/geometry")
        .join(name);
    fs::read_to_string(path).expect("fixture exists")
}

// frob:tests crates/cadetta-geometry/src/placement.rs::g002_on_cycle kind=integration
#[test]
fn geometry_codes_reach_check_and_stay_quiet_on_the_clean_twins() {
    for (code, stem) in [
        ("G002", "g002-two-part-cycle"),
        ("G003", "g003-convert-unplaced-part"),
        ("G004", "g004-aligned-count"),
        ("G006", "g006-part-placed-twice"),
        ("G007", "g007-unplaced-instance"),
    ] {
        let fire = hits(stem, &fixture(&format!("{stem}.must-fire.cdt")), code);
        assert!(!fire.is_empty(), "{code} fires for {stem}");
        let quiet = hits(stem, &fixture(&format!("{stem}.must-not-fire.cdt")), code);
        assert!(quiet.is_empty(), "{code} is quiet for the twin of {stem}");
    }
}
