//! frob:ticket 01M4Y77294QQTRF5TA9H360TSR
//! The pipeline evaluates each part slot alternate and feeds the per-alternate verdicts to I002.

use std::fs;

use cadetta::pipeline::{Analysis, analyze_at};
use cadetta::project::Project;

const SRC: &str = "cite src: web { url = \"https://x.example/a.pdf\" }
pub interface Caster {
  load_rating >= 100N
}
pub ots Good: Caster @src {
  load_rating = 500N
}
pub ots Weak: Caster @src {
  load_rating = 300N
}
pub assembly Rover {
  part wheel: Caster = Good alt [Weak]
  check load_ok: wheel.load_rating >= 440N
}
";

/// Writes a one-package project and analyzes it.
fn analyze_src(tag: &str, src: &str) -> Analysis {
    let dir = std::env::temp_dir().join(format!("cadetta-altverd-{tag}-{}", std::process::id()));
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

// frob:tests crates/cadetta-cli/src/pipeline.rs::alternate_evaluations kind=integration
#[test]
fn alternate_verdicts_fill_per_alternate_and_raise_i002() {
    let a = analyze_src("i002", SRC);
    let ir = &a.packages[0].ir;
    let check = ir
        .checks
        .iter()
        .find(|c| c.path.to_string().ends_with("load_ok"))
        .expect("load_ok lowered");
    assert_eq!(
        check.verdicts.per_alternate.len(),
        1,
        "one entry per alternate: {:?}",
        check.verdicts
    );
    let (key, _) = check.verdicts.per_alternate.iter().next().expect("entry");
    assert!(
        key.ends_with("wheel=acme.demo::Weak"),
        "key is slot=ctor: {key}"
    );
    assert!(
        a.diagnostics.iter().any(|d| d.code == "I002"),
        "I002 for the failing alternate: {:?}",
        a.diagnostics.iter().map(|d| &d.code).collect::<Vec<_>>()
    );
}

// frob:tests crates/cadetta-cli/src/pipeline.rs::alternate_evaluations kind=integration
#[test]
fn a_holding_alternate_raises_no_i002() {
    let src = SRC.replace("= 300N", "= 450N");
    let a = analyze_src("i002ok", &src);
    assert!(
        !a.diagnostics.iter().any(|d| d.code == "I002"),
        "{:?}",
        a.diagnostics.iter().map(|d| &d.code).collect::<Vec<_>>()
    );
}
