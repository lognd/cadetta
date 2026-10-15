//! frob:ticket 01M4VQXP7EE3BE5VQAPMHTX343
//! Exact-zero normalisation and exact trig results raise no Q006 (quantities.md 1, 4.3).

use std::fs;

use cadetta::pipeline::{analyze_at, Analysis};
use cadetta::project::Project;

/// Writes a one-package project whose model `P` holds `body` and analyzes it.
fn analyze_body(tag: &str, body: &str) -> Analysis {
    let dir = std::env::temp_dir().join(format!("cadetta-zero-trig-{tag}-{}", std::process::id()));
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

// frob:tests crates/cadetta-sema/src/eval.rs::binary kind=integration
#[test]
fn exact_zero_then_rad_sum_is_not_q006_but_deg_rad_is() {
    let clean = codes("zero", "z = 1deg - 1deg\n  b = z + 1rad");
    assert_eq!(count(&clean, "Q006"), 0, "{clean:?}");
    let inexact = codes("degrad", "b = 1deg + 1rad");
    assert_eq!(count(&inexact, "Q006"), 1, "{inexact:?}");
}

// frob:tests crates/cadetta-sema/src/func.rs::to_count kind=integration
#[test]
fn exact_trig_points_raise_no_q006() {
    let c = codes("trig-ok", "a = sin(90deg)\n  b = cos(60deg)\n  e = cos(180deg)\n  t = tan(45deg)\n  s = sin(30deg)");
    assert_eq!(count(&c, "Q006"), 0, "{c:?}");
    assert_eq!(count(&c, "EVAL"), 0, "{c:?}");
}

// frob:tests crates/cadetta-sema/src/func.rs::to_count kind=integration
#[test]
fn exact_atan2_points_raise_no_q006() {
    let c = codes("atan2-ok", "a = atan2(3in, 3in)\n  b = atan2(0mm, 2mm)");
    assert_eq!(count(&c, "Q006"), 0, "{c:?}");
}

// frob:tests crates/cadetta-sema/src/func.rs::to_count kind=integration
#[test]
fn inexact_trig_point_raises_q006() {
    let c = codes("trig-bad", "a = sin(91deg)");
    assert_eq!(count(&c, "Q006"), 1, "{c:?}");
}
