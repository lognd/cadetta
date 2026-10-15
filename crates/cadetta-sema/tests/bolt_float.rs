//! Bolt float lowering: `ScrewJoint` at/onto become the placement edge's float (geometry.md 5.2).

// frob:ticket 01M4Y2Z95B99FTVX68T4XTSTXR

use cadetta_sema::eval::{Evaluation, Evaluator};
use cadetta_sema::resolve::{ModuleSrc, PackageSrc, Resolver};
use cadetta_syntax::parser::parse;
use cadetta_units::UnitTable;
use cadetta_units::compare::Verdict;

/// A stand-in `ScrewJoint` (package `std`): the real joint's `at`, `onto`, `float`, `locating` and
/// its own `aligned` check, without the fastener tables.
const JOINT: &str =
    "pub assembly ScrewJoint(at?: Pattern2, onto?: Pattern2, locating: Bool = false) {
  float = 0.5mm
  check aligned: aligned(at, onto, float) \"holes line up\"
}
";

/// The parts a joint sits between.
const PARTS: &str = "model Lid {
  holes: [Point2 on origin; 2] = rect_pattern(2, 1, 30mm, 0mm)
  datum edge = origin.offset(x = 50mm)
  datum mount = origin.offset(x = 5mm)
}
model Base {
  datum top = origin.offset(z = 10mm)
  datum under = origin.offset(z = -10mm)
  tapped: [Point2 on origin; 2] = rect_pattern(2, 1, 30mm, 0mm)
  datum edge = origin.offset(x = 50mm)
}
";

/// Evaluates `JOINT`, `PARTS` and `assembly` as one package named `std`.
fn run(assembly: &str) -> Evaluation {
    let p = parse(&format!("{JOINT}{PARTS}{assembly}"));
    assert!(p.diags.is_empty(), "{:?}", p.diags);
    let pkgs = vec![PackageSrc {
        defaults: Default::default(),
        name: "std".to_owned(),
        deps: Vec::new(),
        modules: vec![ModuleSrc {
            path: Vec::new(),
            file: &p.file,
            enums: &[],
        }],
    }];
    let (resolver, errs) = Resolver::new(pkgs.clone());
    assert!(errs.is_empty());
    let units = UnitTable::builtin();
    Evaluator::new(&resolver, &units, &pkgs).evaluate()
}

/// The verdict of check `name`.
fn verdict(ev: &Evaluation, name: &str) -> Verdict {
    ev.checks
        .iter()
        .find(|c| c.path.to_string().ends_with(name))
        .unwrap_or_else(|| panic!("no check {name}: {:?}", ev.checks))
        .verdict
}

/// A box of `lid` over `base` with the lid or the base as the placed child.
fn boxed(lid_child: bool, joints: &str, check: &str) -> String {
    let placement = if lid_child {
        "place base.origin at origin\n  place lid.origin at base.top"
    } else {
        "place lid.origin at origin\n  place base.origin at lid.origin"
    };
    format!(
        "assembly Box {{\n  part lid: Lid\n  part base: Base\n  {placement}\n  {joints}\n  {check}\n}}\n"
    )
}

const J: &str = "part j: ScrewJoint(at = lid.holes, onto = base.tapped)";
const FLUSH: &str =
    "check flush: (lid.edge in origin).x - (base.edge in origin).x <= 0.1mm \"flush\"";

// frob:tests crates/cadetta-sema/tests/bolt_float.rs::floating_joint_frees_the_child_in_either_order kind=unit
#[test]
fn floating_joint_frees_the_child_in_either_order() {
    for lid_child in [true, false] {
        let ev = run(&boxed(lid_child, J, FLUSH));
        // only the undecided flush check (C002) is reported
        assert!(
            ev.diagnostics
                .iter()
                .all(|d| d.code == "C002" || d.code == "G005"),
            "{:?}",
            ev.diagnostics
        );
        assert_eq!(
            verdict(&ev, "flush"),
            Verdict::Undecided,
            "lid_child={lid_child}"
        );
    }
}

// frob:tests crates/cadetta-sema/tests/bolt_float.rs::float_applies_in_the_placement_datum_frame kind=unit
#[test]
fn float_applies_in_the_placement_datum_frame() {
    let shifted =
        "check flush: (base.edge in origin).x - (lid.edge in origin).x <= 5.2mm \"shift\"";
    let src = boxed(true, J, shifted).replace("place lid.origin at", "place lid.mount at");
    let ev = run(&src);
    // the 5mm mount shift also (rightly) violates the joint's own 0.5mm alignment (C001)
    assert!(
        ev.diagnostics
            .iter()
            .all(|d| matches!(d.code, "C002" | "C001" | "G005")),
        "{:?}",
        ev.diagnostics
    );
    assert_eq!(verdict(&ev, "flush"), Verdict::Undecided);
}

// frob:tests crates/cadetta-sema/tests/bolt_float.rs::locating_joint_keeps_the_child_fixed kind=unit
#[test]
fn locating_joint_keeps_the_child_fixed() {
    for lid_child in [true, false] {
        let src = boxed(
            lid_child,
            "part j: ScrewJoint(at = lid.holes, onto = base.tapped, locating = true)",
            FLUSH,
        );
        let ev = run(&src);
        assert!(ev.diagnostics.is_empty(), "{:?}", ev.diagnostics);
        assert_eq!(
            verdict(&ev, "flush"),
            Verdict::Holds,
            "lid_child={lid_child}"
        );
    }
}

// frob:tests crates/cadetta-sema/tests/bolt_float.rs::joints_on_one_edge_reach_select_together kind=unit
#[test]
fn joints_on_one_edge_reach_select_together() {
    let two = "part j: ScrewJoint(at = lid.holes, onto = base.tapped)\n  \
               part k: ScrewJoint(at = lid.holes, onto = base.tapped, locating = true)";
    let ev = run(&boxed(true, two, FLUSH));
    assert!(ev.diagnostics.is_empty(), "{:?}", ev.diagnostics);
    // one locating joint on the edge removes the float of the other
    assert_eq!(verdict(&ev, "flush"), Verdict::Holds);
}

// frob:tests crates/cadetta-sema/tests/bolt_float.rs::own_aligned_check_reads_the_nominal_placement kind=unit
#[test]
fn own_aligned_check_reads_the_nominal_placement() {
    let ev = run(&boxed(true, J, FLUSH));
    assert_eq!(verdict(&ev, "aligned"), Verdict::Holds);
}

// frob:tests crates/cadetta-sema/tests/bolt_float.rs::joint_between_siblings_adds_no_float kind=unit
#[test]
fn joint_between_siblings_adds_no_float() {
    let src = "assembly Box {
  part lid: Lid
  part base: Base
  part top: Lid
  place base.origin at origin
  place lid.origin at base.top
  place top.origin at base.top
  part j: ScrewJoint(at = lid.holes, onto = top.holes)
  check flush: (lid.edge in origin).x - (top.edge in origin).x <= 0.1mm \"flush\"
}
";
    let ev = run(src);
    assert!(ev.diagnostics.is_empty(), "{:?}", ev.diagnostics);
    assert_eq!(verdict(&ev, "flush"), Verdict::Holds);
}

// frob:tests crates/cadetta-sema/tests/bolt_float.rs::g005_fires_for_a_check_tighter_than_the_float kind=unit
#[test]
fn g005_fires_for_a_check_tighter_than_the_float() {
    for lid_child in [true, false] {
        let ev = run(&boxed(lid_child, J, FLUSH));
        let n = ev.diagnostics.iter().filter(|d| d.code == "G005").count();
        assert_eq!(n, 1, "lid_child={lid_child}: {:?}", ev.diagnostics);
    }
}

// frob:tests crates/cadetta-sema/tests/bolt_float.rs::g005_stays_quiet_when_locating_or_looser kind=unit
#[test]
fn g005_stays_quiet_when_locating_or_looser() {
    let loose = "check flush: (lid.edge in origin).x - (base.edge in origin).x <= 2mm \"flush\"";
    let ev = run(&boxed(true, J, loose));
    assert_eq!(verdict(&ev, "flush"), Verdict::Holds);
    assert!(
        ev.diagnostics.iter().all(|d| d.code != "G005"),
        "{:?}",
        ev.diagnostics
    );
    let loc = "part j: ScrewJoint(at = lid.holes, onto = base.tapped, locating = true)";
    let ev = run(&boxed(true, loc, FLUSH));
    assert!(
        ev.diagnostics.iter().all(|d| d.code != "G005"),
        "{:?}",
        ev.diagnostics
    );
}

/// A box whose joint has a one-point pattern, so it cannot fix rotation.
const ONE_POINT: &str = "model Lid1 {
  holes: [Point2 on origin; 1] = rect_pattern(1, 1, 30mm, 0mm)
  datum edge = origin.offset(x = 50mm)
}
model Base1 {
  datum top = origin.offset(z = 10mm)
  tapped: [Point2 on origin; 1] = rect_pattern(1, 1, 30mm, 0mm)
  datum edge = origin.offset(x = 50mm)
}
assembly Box1 {
  part lid: Lid1
  part base: Base1
  place base.origin at origin
  place lid.origin at base.top
  JOINT
  check flush: (lid.edge in origin).x - (base.edge in origin).x <= 0.1mm \"flush\"
}
";

// frob:tests crates/cadetta-sema/src/geom.rs::report_unconstrained_rotation kind=unit
#[test]
fn g008_fires_once_for_a_one_point_non_locating_joint() {
    let joint = "part j: ScrewJoint(at = lid.holes, onto = base.tapped)";
    let ev = run(&ONE_POINT.replace("JOINT", joint));
    let n = ev.diagnostics.iter().filter(|d| d.code == "G008").count();
    assert_eq!(n, 1, "{:?}", ev.diagnostics);
    let loc = "part j: ScrewJoint(at = lid.holes, onto = base.tapped, locating = true)";
    let ev = run(&ONE_POINT.replace("JOINT", loc));
    assert!(
        ev.diagnostics.iter().all(|d| d.code != "G008"),
        "{:?}",
        ev.diagnostics
    );
}

// frob:tests crates/cadetta-sema/src/geom.rs::report_unconstrained_rotation kind=unit
#[test]
fn g008_stays_quiet_for_a_two_point_joint() {
    let ev = run(&boxed(true, J, FLUSH));
    assert!(
        ev.diagnostics.iter().all(|d| d.code != "G008"),
        "{:?}",
        ev.diagnostics
    );
}
