//! frob:ticket 01M4V1HDJS2MDQF8GRK4QX9B1C
//! One fault, one diagnostic: the lexer, kind checker and evaluator do not double-report
//! (~4QX9B1C), and package-declared kinds are distinct Q003 targets (~Q6AFN5S, ~8S2ESWE).

use std::fs;

use cadetta::pipeline::{Analysis, analyze_at};
use cadetta::project::Project;

/// Writes a one-package project with `src` as `src/lib.cdt` and analyzes it.
fn analyze_src(tag: &str, src: &str) -> Analysis {
    let dir = std::env::temp_dir().join(format!("cadetta-diag-once-{tag}-{}", std::process::id()));
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

/// Error/warning codes of `src`, in order (info-level lints excluded).
fn codes(tag: &str, src: &str) -> Vec<String> {
    analyze_src(tag, src)
        .diagnostics
        .iter()
        .filter(|d| d.code != "U002")
        .map(|d| d.code.to_string())
        .collect()
}

// frob:tests crates/cadetta-cli/src/pipeline.rs::analyze_at kind=integration
#[test]
fn lexical_error_reports_one_e001() {
    for (i, lit) in ["2ft3kg", "5.mm", ".5mm", "2.5ft6in", "6in2ft"]
        .iter()
        .enumerate()
    {
        let c = codes(
            &format!("e001-{i}"),
            &format!("model P {{\n  a = {lit}\n}}\n"),
        );
        assert_eq!(c, vec!["E001"], "{lit}");
    }
}

// frob:tests crates/cadetta-cli/src/pipeline.rs::analyze_at kind=integration
#[test]
fn kind_fault_shadows_evaluator_report() {
    let cases = [
        ("unit", "a = 5foo", "E002"),
        ("operand", "a = 5mm + 3", "Q001"),
        ("trig", "a = sin(5mm)", "Q002"),
        ("round", "a = round(5mm, 2s)", "Q002"),
    ];
    for (tag, body, want) in cases {
        let c = codes(tag, &format!("model P {{\n  {body}\n}}\n"));
        assert_eq!(
            c.iter().filter(|x| x.as_str() == want).count(),
            1,
            "{body}: {c:?}"
        );
        assert!(!c.iter().any(|x| x == "EVAL"), "{body}: {c:?}");
    }
}

// frob:tests crates/cadetta-cli/src/pipeline.rs::analyze_at kind=integration
#[test]
fn check_condition_is_kind_checked_once_with_a_path() {
    let a = analyze_src("check", "model P {\n  x = 3mm\n  check c: x >= 5\n}\n");
    let q: Vec<_> = a.diagnostics.iter().filter(|d| d.code == "Q001").collect();
    assert_eq!(q.len(), 1, "{:?}", a.diagnostics);
    assert_eq!(q[0].path.as_deref(), Some("acme.demo::P.c"));
}

// frob:tests crates/cadetta-cli/src/pipeline.rs::analyze_at kind=integration
#[test]
fn package_kind_is_distinct_and_explicit_conversion_is_quiet() {
    let head = "kind Stress = Pressure\nmodel Part {\n  s: Stress = 5MPa as Stress\n";
    let fire = codes("q003-fire", &format!("{head}  p: Pressure = s\n}}\n"));
    assert_eq!(fire.iter().filter(|c| *c == "Q003").count(), 1, "{fire:?}");
    let quiet = codes(
        "q003-quiet",
        &format!("{head}  p: Pressure = s as Pressure\n}}\n"),
    );
    assert!(!quiet.iter().any(|c| c == "Q003"), "{quiet:?}");
    let bare = codes(
        "q003-bare",
        "kind Stress = Pressure\nmodel P {\n  s: Stress = 5MPa\n}\n",
    );
    assert_eq!(bare.iter().filter(|c| *c == "Q003").count(), 1, "{bare:?}");
}
