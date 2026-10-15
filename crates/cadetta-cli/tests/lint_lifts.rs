//! frob:ticket 01M4TRKDAC6FR44KEW64H7PTZX
//! Q007, P001, F001-F003, T001 and L001/L002 reach `cadetta check` through `analyze` (~4H7PTZX).

use std::fs;
use std::path::PathBuf;

use cadetta::pipeline::{Analysis, analyze_at};
use cadetta::project::Project;
use cadetta_lint::diag::Severity;

/// Writes a one-package project with `src` as `src/lib.cdt` and analyzes it at `as_of`.
fn analyze_src(tag: &str, src: &str, as_of: &str) -> Analysis {
    let dir: PathBuf =
        std::env::temp_dir().join(format!("cadetta-lint-lifts-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("src")).expect("mkdir");
    fs::write(
        dir.join("cadetta.toml"),
        "[package]\nname = \"acme.demo\"\nversion = \"0.1.0\"\n",
    )
    .expect("config");
    fs::write(dir.join("src/lib.cdt"), src).expect("source");
    let (project, scope) = Project::open(Some(&dir), None).expect("project opens");
    let analysis = analyze_at(&project, &scope, as_of);
    let _ = fs::remove_dir_all(&dir);
    analysis
}

fn run(tag: &str, src: &str) -> Analysis {
    analyze_src(tag, src, "2026-10-14")
}

fn codes(a: &Analysis) -> Vec<(String, Option<String>)> {
    a.diagnostics
        .iter()
        .map(|d| (d.code.clone(), d.path.clone()))
        .collect()
}

fn has(a: &Analysis, code: &str) -> bool {
    a.diagnostics.iter().any(|d| d.code == code)
}

const CASTER: &str = "cite src: web { url = \"https://x.example/a.pdf\" }
pub interface Caster {
  diameter: Length
  load_rating >= 400N
}
pub ots Good: Caster @src {
  diameter = 5in
  load_rating = 500N
}
";

fn rover(old_attr: &str) -> String {
    format!(
        "{CASTER}{old_attr}\npub ots Old: Caster @src {{\n  diameter = 5in\n  load_rating = 500N\n}}\npub assembly Rover {{\n  part wheel: Caster = Old\n}}\n"
    )
}

// frob:tests crates/cadetta-cli/src/pipeline.rs::parse_package kind=unit
#[test]
fn q007_fires_as_a_fixable_error_for_a_split_quantity() {
    let a = run("q007", "model M {\n  width = 5 mm\n}\n");
    let d = a
        .diagnostics
        .iter()
        .find(|d| d.code == "Q007")
        .unwrap_or_else(|| panic!("{:?}", codes(&a)));
    assert_eq!(d.severity, Severity::Error);
    assert!(!d.fixes.is_empty(), "Q007 carries the join fix");
}

// frob:tests crates/cadetta-cli/src/pipeline.rs::parse_package kind=unit
#[test]
fn q007_is_quiet_for_an_attached_unit() {
    let a = run("q007ok", "model M {\n  width = 5mm\n}\n");
    assert!(!has(&a, "Q007"), "{:?}", codes(&a));
}

// frob:tests crates/cadetta-cli/src/pipeline.rs::evaluate_stages kind=unit
#[test]
fn p001_fires_for_a_feature_without_a_role_and_not_with_one() {
    let proc =
        "process Fdm {\n  hole_comp = 0.2mm\n  rule hole(d: Length) -> Length = d + hole_comp\n}\n";
    let bad = run(
        "p001",
        &format!("{proc}model Case {{\n  #![process(Fdm)]\n  insert_hole = 3.4mm\n}}\n"),
    );
    assert!(has(&bad, "P001"), "{:?}", codes(&bad));
    let good = run(
        "p001ok",
        &format!("{proc}model Case {{\n  #![process(Fdm)]\n  insert_hole = 3.4mm  #[hole]\n}}\n"),
    );
    assert!(!has(&good, "P001"), "{:?}", codes(&good));
}

// frob:tests crates/cadetta-sema/src/fitderive.rs::derive_fits_with_findings kind=unit
#[test]
fn f001_and_f002_fire_for_a_bad_class_and_an_out_of_range_size() {
    let f001 = run(
        "f001",
        "model Part {\n  bore = 20mm\n  fit seat = H19/p6 on bore\n}\n",
    );
    assert!(has(&f001, "F001"), "{:?}", codes(&f001));
    let f002 = run(
        "f002",
        "model Part {\n  bore = 600mm\n  fit seat = H7/p6 on bore\n}\n",
    );
    assert!(has(&f002, "F002"), "{:?}", codes(&f002));
}

// frob:tests crates/cadetta-sema/src/fitderive.rs::derive_fits_with_findings kind=unit
#[test]
fn f001_and_f002_are_quiet_for_a_valid_fit() {
    let a = run(
        "fok",
        "model Part {\n  bore = 20mm\n  fit seat = H7/p6 on bore\n}\n",
    );
    assert!(
        !has(&a, "F001") && !has(&a, "F002") && !has(&a, "F003"),
        "{:?}",
        codes(&a)
    );
}

// frob:tests crates/cadetta-sema/src/fitderive.rs::derive_fits_with_findings kind=unit
#[test]
fn f003_fires_when_a_side_tolerance_disagrees_with_its_class() {
    let bad = run(
        "f003",
        "model Part {\n  bore = 20mm [+-0.5mm]\n  fit seat = H7/p6 on bore\n}\n",
    );
    assert!(has(&bad, "F003"), "{:?}", codes(&bad));
}

const TABLE: &str = "use std::iso273
table Iso273 @iso273 {
  key d: Length
  cols pitch: Length
  5mm => 0.8mm
  6mm => 1mm
}
";

// frob:tests crates/cadetta-cli/src/pipeline.rs::table_misses kind=unit
#[test]
fn t001_fires_for_a_literal_matching_no_row_and_not_for_a_match() {
    let miss = run(
        "t001",
        &format!("{TABLE}pub ots Screw @iso273 {{\n  major = 5mm\n  pitch = 0.9mm\n}}\n"),
    );
    let d = miss
        .diagnostics
        .iter()
        .find(|d| d.code == "T001")
        .unwrap_or_else(|| panic!("{:?}", codes(&miss)));
    assert!(d.path.as_deref().is_some_and(|p| p.contains("Screw.pitch")));
    let hit = run(
        "t001ok",
        &format!("{TABLE}pub ots Screw @iso273 {{\n  major = 5mm\n  pitch = 0.8mm\n}}\n"),
    );
    assert!(!has(&hit, "T001"), "{:?}", codes(&hit));
}

// frob:tests crates/cadetta-variants/src/lifecycle.rs::analyse kind=unit
#[test]
fn l001_is_an_error_once_the_date_has_passed_and_a_warning_before() {
    let src = rover("#[lifecycle(eol, \"2026-09\")]");
    let late = analyze_src("l001late", &src, "2026-10-14");
    let d = late
        .diagnostics
        .iter()
        .find(|d| d.code == "L001")
        .unwrap_or_else(|| panic!("{:?}", codes(&late)));
    assert_eq!(d.severity, Severity::Error);
    let early = analyze_src("l001early", &src, "2026-01-01");
    let d = early
        .diagnostics
        .iter()
        .find(|d| d.code == "L001")
        .unwrap_or_else(|| panic!("{:?}", codes(&early)));
    assert_eq!(d.severity, Severity::Warning);
}

// frob:tests crates/cadetta-variants/src/lifecycle.rs::analyse kind=unit
#[test]
fn l001_is_quiet_for_an_active_primary_or_an_eol_one_with_a_viable_alternate() {
    let active = run("l001act", &rover("#[lifecycle(active, \"2026-09\")]"));
    assert!(!has(&active, "L001"), "{:?}", codes(&active));
    let src = rover("#[lifecycle(eol, \"2026-09\")]").replace("= Old\n", "= Old alt [Good]\n");
    let alt = run("l001alt", &src);
    assert!(!has(&alt, "L001"), "{:?}", codes(&alt));
}

// frob:tests crates/cadetta-variants/src/lifecycle.rs::analyse kind=unit
#[test]
fn l002_reports_a_not_recommended_primary() {
    let a = run("l002", &rover("#[lifecycle(nrnd, \"2026-09\")]"));
    assert!(has(&a, "L002"), "{:?}", codes(&a));
}

/// An eol `Old` primary whose `Big` alternate is wider than the assembly's check allows.
fn swap_rover(alt: &str) -> String {
    format!(
        "{CASTER}#[lifecycle(eol, \"2026-09\")]\npub ots Old: Caster @src {{\n  diameter = 5in\n  load_rating = 500N\n}}\npub ots Big: Caster @src {{\n  diameter = 8in\n  load_rating = 500N\n}}\npub assembly Rover {{\n  part wheel: Caster = Old alt [{alt}]\n  check fits: wheel.diameter <= 6in\n}}\n"
    )
}

// frob:ticket 01M4YMYMD920CQR97CW6QJCHN0
// frob:tests crates/cadetta-cli/src/pipeline.rs::lifecycle_findings kind=integration
#[test]
fn l001_variant_evaluation_rejects_an_alternate_that_violates_a_check() {
    let bad = run("l001varbad", &swap_rover("Big"));
    assert!(
        has(&bad, "L001"),
        "violating alternate is not viable: {:?}",
        codes(&bad)
    );
    let good = run("l001vargood", &swap_rover("Good"));
    assert!(!has(&good, "L001"), "{:?}", codes(&good));
}
