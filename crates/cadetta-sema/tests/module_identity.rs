//! Resolve, evaluation and dependency diagnostics carry the module their span lies in.

// frob:ticket 01M4TA3BTW256J8FKMS6KASY82

use cadetta_sema::deps::{DepGraph, Renames};
use cadetta_sema::eval::Evaluator;
use cadetta_sema::resolve::{ModuleKey, ModuleSrc, PackageSrc, Resolver};
use cadetta_syntax::ast::File;
use cadetta_syntax::parser::parse;
use cadetta_units::UnitTable;

/// Parses `src`, asserting no diagnostics.
fn file(src: &str) -> File {
    let p = parse(src);
    assert!(p.diags.is_empty(), "{:?}", p.diags);
    p.file
}

/// One package `acme.rover` whose modules are the given `(path, file)` pairs.
fn pkgs<'a>(mods: &[(&str, &'a File)]) -> Vec<PackageSrc<'a>> {
    vec![PackageSrc {
        defaults: Default::default(),
        name: "acme.rover".to_owned(),
        deps: Vec::new(),
        modules: mods
            .iter()
            .map(|(p, f)| ModuleSrc {
                path: vec![(*p).to_owned()],
                file: f,
                enums: &[],
            })
            .collect(),
    }]
}

/// The key of module `acme.rover::<m>`.
fn key(m: &str) -> Option<ModuleKey> {
    Some(ModuleKey::new("acme.rover", &[m.to_owned()]))
}

// frob:tests resolve_errors_carry_their_module
#[test]
fn resolve_errors_carry_their_module() {
    let a = file("model Fine { x = 1 }\n");
    let b = file("model A { x = 1 }\nmodel A { y = 1 }\n");
    let (_r, errs) = Resolver::new(pkgs(&[("alpha", &a), ("frame", &b)]));
    assert_eq!(errs.len(), 1);
    assert_eq!(errs[0].code, "E003");
    assert_eq!(errs[0].module.as_deref(), key("frame").as_ref());
}

// frob:tests eval_diagnostics_carry_their_module
#[test]
fn eval_diagnostics_carry_their_module() {
    let a = file("model Fine { x = 1 }\n");
    let b = file("model Bad { y = nothing_here + 1 }\n");
    let p = pkgs(&[("alpha", &a), ("frame", &b)]);
    let (resolver, _errs) = Resolver::new(p.clone());
    let units = UnitTable::builtin();
    let ev = Evaluator::new(&resolver, &units, &p).evaluate();
    assert!(!ev.diagnostics.is_empty());
    for d in &ev.diagnostics {
        assert_eq!(d.module, key("frame"), "{d:?}");
    }
}

// frob:tests dep_diagnostics_carry_their_module
#[test]
fn dep_diagnostics_carry_their_module() {
    let a = file("model Fine { x = 1 }\n");
    let b = file("model M {\n  a = b + 1\n  b = a + 1\n}\n");
    let p = pkgs(&[("alpha", &a), ("frame", &b)]);
    let (resolver, errs) = Resolver::new(p.clone());
    assert!(errs.is_empty());
    let g = DepGraph::build(&resolver, &Renames::collect(&resolver, &p), &p);
    let d = g.diagnostics();
    assert_eq!(d.len(), 1, "{d:?}");
    assert_eq!(d[0].code, "D003");
    assert_eq!(d[0].module, key("frame"));
}

// frob:tests renamed_reference_warning_carries_the_using_module
#[test]
fn renamed_reference_warning_carries_the_using_module() {
    let a = file("model Globals {\n  plate_thickness = 6mm was Globals.plate_thk\n}\n");
    let b = file("use acme.rover::alpha::Globals\nmodel User {\n  t = Globals.plate_thk\n}\n");
    let p = pkgs(&[("alpha", &a), ("frame", &b)]);
    let (resolver, errs) = Resolver::new(p.clone());
    assert!(errs.is_empty(), "{errs:?}");
    let g = DepGraph::build(&resolver, &Renames::collect(&resolver, &p), &p);
    let d: Vec<_> = g
        .diagnostics()
        .iter()
        .filter(|d| d.code == "N004")
        .collect();
    assert_eq!(d.len(), 1, "{:?}", g.diagnostics());
    assert_eq!(d[0].module, key("frame"));
}
