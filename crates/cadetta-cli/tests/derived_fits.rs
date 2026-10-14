//! frob:ticket 01M4TA3C37X4GWR2YBJQ1ZBN9Y
//! The pipeline derives fit declarations into lowered IR and the F004 lint sees them.

use std::fs;
use std::path::PathBuf;

use cadetta::pipeline::{Analysis, analyze};
use cadetta::project::Project;

/// Writes a one-package project with `src` as `src/lib.cdt` and analyzes it.
fn analyze_src(tag: &str, src: &str) -> Analysis {
    let dir: PathBuf =
        std::env::temp_dir().join(format!("cadetta-derived-fits-{tag}-{}", std::process::id()));
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

// frob:tests crates/cadetta-sema/src/fitderive.rs::derive_fits kind=unit
#[test]
fn range_fit_reaches_the_ir_with_required_bound_and_fires_f004() {
    let analysis = analyze_src(
        "range",
        "model Part {\n  bore = 20mm\n  fit seat = H7/p6 on bore [-0.4mm .. -0.1mm]\n}\n",
    );
    assert!(!analysis.halted);
    let comp = analysis.packages[0]
        .ir
        .components
        .iter()
        .find(|c| c.path.to_string().ends_with("Part"))
        .expect("component lowered");
    assert_eq!(comp.fits.len(), 1, "fits: {:?}", comp.fits);
    let fit = &comp.fits[0];
    assert!(fit.path.to_string().ends_with("seat"));
    assert_eq!(fit.fit_type, "interference");
    assert!(fit.required.is_some());
    assert!(
        analysis.diagnostics.iter().any(|d| d.code == "F004"),
        "diagnostics: {:?}",
        analysis
            .diagnostics
            .iter()
            .map(|d| &d.code)
            .collect::<Vec<_>>()
    );
}

// frob:tests crates/cadetta-sema/src/fitderive.rs::derive_fits kind=unit
#[test]
fn satisfied_range_fit_is_lowered_without_f004() {
    let analysis = analyze_src(
        "ok",
        "model Part {\n  bore = 20mm\n  fit seat = H7/p6 on bore [-0.1mm .. 0.1mm]\n}\n",
    );
    let comp = &analysis.packages[0].ir.components[0];
    assert_eq!(comp.fits.len(), 1);
    assert!(analysis.diagnostics.iter().all(|d| d.code != "F004"));
}

// frob:tests crates/cadetta-sema/src/fitderive.rs::derive_fits kind=unit
#[test]
fn class_fit_on_another_components_member_is_derived() {
    let analysis = analyze_src(
        "class",
        "model Tube {\n  od = 20mm [+-0.05mm]\n}\nmodel Bracket {\n  fit seat = H11 on Tube.od\n}\n",
    );
    let comp = analysis.packages[0]
        .ir
        .components
        .iter()
        .find(|c| c.path.to_string().ends_with("Bracket"))
        .expect("component lowered");
    assert_eq!(
        comp.fits.len(),
        1,
        "diagnostics: {:?}",
        analysis
            .diagnostics
            .iter()
            .map(|d| (&d.code, &d.message))
            .collect::<Vec<_>>()
    );
    assert_eq!(comp.fits[0].fit_type, "transition");
    assert!(
        comp.fits[0].clearance.tol.is_some(),
        "clearance carries the tolerance band"
    );
    assert!(comp.fits[0].required.is_none());
}
