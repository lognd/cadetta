//! frob:ticket 01M4V1HE7FVQECRVX6S7K6PYDX
//! frob:ticket 01M4V1HEA425D4RGFBSG5NT80Z
//! Pi-factor floor (Q006), mixed-pi sums, compare (Q008) and inexact check operands (C002).

use std::fs;

use cadetta::pipeline::{analyze_at, Analysis};
use cadetta::project::Project;

/// Writes a one-package project whose model `P` holds `body` and analyzes it.
fn analyze_body(tag: &str, body: &str) -> Analysis {
    let dir = std::env::temp_dir().join(format!("cadetta-pi-inexact-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("src")).expect("mkdir");
    fs::write(
        dir.join("cadetta.toml"),
        "[package]\nname = \"acme.demo\"\nversion = \"0.1.0\"\n",
    )
    .expect("config");
    fs::write(dir.join("src/lib.cdt"), format!("model P {{\n  {body}\n}}\n")).expect("source");
    let (project, scope) = Project::open(Some(&dir), None).expect("project opens");
    let analysis = analyze_at(&project, &scope, "2026-10-14");
    let _ = fs::remove_dir_all(&dir);
    analysis
}

/// Diagnostic codes of `body`, in order.
fn codes(tag: &str, body: &str) -> Vec<String> {
    analyze_body(tag, body)
        .diagnostics
        .iter()
        .map(|d| d.code.to_string())
        .collect()
}

fn count(c: &[String], code: &str) -> usize {
    c.iter().filter(|x| x.as_str() == code).count()
}

// frob:tests crates/cadetta-sema/src/func.rs::to_count kind=integration
#[test]
fn floor_of_pi_carrying_value_raises_q006() {
    let c = codes("floor-pi", "a = floor(1rev / 1rad)");
    assert_eq!(count(&c, "Q006"), 1, "{c:?}");
    assert_eq!(count(&c, "EVAL"), 0, "{c:?}");
}

// frob:tests crates/cadetta-sema/src/func.rs::to_count kind=integration
#[test]
fn floor_of_pi_free_ratio_is_clean() {
    let c = codes("floor-clean", "a = floor(360deg / 1rev)");
    assert_eq!(count(&c, "Q006"), 0, "{c:?}");
    assert_eq!(count(&c, "EVAL"), 0, "{c:?}");
}

// frob:tests crates/cadetta-sema/src/eval.rs::binary kind=integration
#[test]
fn deg_plus_rad_is_inexact_not_eval() {
    let c = codes("deg-rad", "b = 1deg + 1rad");
    assert_eq!(count(&c, "EVAL"), 0, "{c:?}");
    assert_eq!(count(&c, "Q006"), 1, "{c:?}");
}

// frob:tests crates/cadetta-sema/src/eval.rs::binary kind=integration
#[test]
fn exact_zero_normalised_sum_is_clean() {
    let c = codes("zero", "z = 1deg - 1deg\n  b = z + 1rad");
    assert_eq!(count(&c, "EVAL"), 0, "{c:?}");
    assert_eq!(count(&c, "Q006"), 0, "{c:?}");
}

// frob:tests crates/cadetta-sema/src/checks.rs::condition kind=integration
#[test]
fn compare_q008_fires_only_without_per_rev() {
    let bad = codes("q008-bad", "check c: 1500rpm * 8mm >= 0.1m/s");
    assert_eq!(count(&bad, "Q008"), 1, "{bad:?}");
    let ok = codes("q008-ok", "check c: 1500rpm * 8mm/rev >= 0.1m/s");
    assert_eq!(count(&ok, "Q008"), 0, "{ok:?}");
}

// frob:tests crates/cadetta-sema/src/checks.rs::condition kind=integration
#[test]
fn inexact_check_operand_on_boundary_is_c002() {
    let c = codes("c002-fire", "check c: sqrt(2mm^2) >= 1.4142135623731mm");
    assert_eq!(count(&c, "EVAL"), 0, "{c:?}");
    assert_eq!(count(&c, "C002"), 1, "{c:?}");
}

// frob:tests crates/cadetta-sema/src/checks.rs::condition kind=integration
#[test]
fn inexact_check_operand_clear_of_boundary_holds() {
    let c = codes("c002-hold", "check c: sqrt(2mm^2) >= 1mm");
    assert_eq!(count(&c, "EVAL"), 0, "{c:?}");
    assert_eq!(count(&c, "C002"), 0, "{c:?}");
}
