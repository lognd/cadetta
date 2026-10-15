//! ScrewJoint visibility over the real std source: priv intermediates are E016 cross-package (packages.md 3).

// frob:ticket 01M4STAPCEF5EPEYX0MZDSBA7J
// frob:ticket 01M4STAPFPQ9GV8XJB4XCK8EGP

use cadetta_sema::resolve::{Ctx, ModuleSrc, PackageSrc, Resolver};
use cadetta_sema::visibility::VisibilityChecker;
use cadetta_syntax::ast::{File, MemberKind};
use cadetta_syntax::parser::parse;

const SRC: &str = include_str!("../../../std/src/fasteners/screwjoint.cdt");

/// Parses `src`, asserting no diagnostics.
fn file(src: &str) -> File {
    let p = parse(src);
    assert!(p.diags.is_empty(), "{:?}", p.diags);
    p.file
}

/// The E016 outcome of a dependent package reading `<member>` of a ScrewJoint instance.
fn read(member: &str) -> Result<(), &'static str> {
    let std_file = file(SRC);
    let user = file(&format!(
        "use std::fasteners::screwjoint::ScrewJoint\nassembly Top {{\n  part s: ScrewJoint\n  v = s.{member}\n}}"
    ));
    let pkgs = vec![
        PackageSrc {
            name: "std".to_owned(),
            deps: Vec::new(),
            modules: vec![ModuleSrc {
                path: vec!["fasteners".to_owned(), "screwjoint".to_owned()],
                file: &std_file,
                enums: &[],
            }],
        },
        PackageSrc {
            name: "q".to_owned(),
            deps: vec!["std".to_owned()],
            modules: vec![ModuleSrc {
                path: Vec::new(),
                file: &user,
                enums: &[],
            }],
        },
    ];
    let (r, _) = Resolver::new(pkgs.clone());
    let v = VisibilityChecker::new(&r, &pkgs);
    let from = Ctx {
        package: "q".to_owned(),
        module: Vec::new(),
        component: Some("Top".to_owned()),
        locals: Vec::new(),
    };
    let expr = match &user.components().next().unwrap().members[1].kind {
        MemberKind::Param(p) => p.value.clone(),
        _ => unreachable!(),
    };
    let res = r.resolve_ref(&from, &expr, None).expect("resolves");
    v.check_use(&from, &res, expr.span).map_err(|e| e.code)
}

// frob:tests crates/cadetta-sema/tests/std_screwjoint.rs::priv_intermediates_are_e016_and_public_members_read kind=unit
#[test]
fn priv_intermediates_are_e016_and_public_members_read() {
    for m in [
        "major",
        "pitch",
        "pocket_dia",
        "pocket_depth",
        "column",
        "drill_nominal",
        "material",
        "n",
        "engagement",
        "depth_needed",
    ] {
        assert_eq!(read(m), Err("E016"), "{m}");
    }
    for m in [
        "clearance_hole",
        "grip",
        "screw_length",
        "tap_drill",
        "drill_depth",
        "callout",
        "float",
        "head",
    ] {
        assert_eq!(read(m), Ok(()), "{m}");
    }
}
