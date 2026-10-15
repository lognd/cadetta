//! Reads through an interface-typed part slot evaluate against the slot's primary component.

// frob:ticket 01M4Y77276XRHD2NDXHJMCB8NQ

use cadetta_sema::eval::Evaluator;
use cadetta_sema::eval::Val;
use cadetta_sema::resolve::{CanonPath, MemberSeg, ModuleSrc, PackageSrc, Resolver};
use cadetta_syntax::parser::parse;
use cadetta_units::compare::Verdict;
use cadetta_units::{Magnitude, Rational, UnitTable};

// frob:tests slot_component
#[test]
fn interface_slot_read_resolves_to_primary() {
    let src = "interface Motor { mass: Mass }\n\
               model MotorA { mass = 2kg }\n\
               model MotorB { mass = 5kg }\n\
               model Rig {\n  part motor: Motor = MotorA alt [MotorB]\n  m = motor.mass\n}\n";
    let p = parse(src);
    assert!(p.diags.is_empty(), "{:?}", p.diags);
    let pk = vec![PackageSrc {
        name: "acme.rover".to_owned(),
        deps: Vec::new(),
        modules: vec![ModuleSrc {
            path: vec!["rig".to_owned()],
            file: &p.file,
            enums: &[],
        }],
    }];
    let (resolver, errs) = Resolver::new(pk.clone());
    assert!(errs.is_empty(), "{errs:?}");
    let units = UnitTable::builtin();
    let ev = Evaluator::new(&resolver, &units, &pk).evaluate();
    assert!(ev.diagnostics.is_empty(), "{:?}", ev.diagnostics);
    let mut at = CanonPath::item("acme.rover", &["rig".to_owned()], "Rig");
    at.members.push(MemberSeg::Name("m".to_owned()));
    let Val::Quantity(q) = &ev.get(&at).expect("m evaluated").value else {
        panic!("m is not a quantity");
    };
    // MotorA (the primary) weighs 2 kg, not MotorB's 5 kg.
    assert_eq!(q.nominal, Magnitude::new(Rational::new(2, 1).expect("denominator"), 0));
}

// frob:tests slot_component
#[test]
fn check_through_interface_slot_yields_a_verdict() {
    let src = "interface Caster { load_rating: Force }\n\
               ots Good: Caster { load_rating = 500N }\n\
               ots Bad: Caster { load_rating = 100N }\n\
               assembly Rover {\n  part wheel: Caster = Good alt [Bad]\n  \
               check load_ok: wheel.load_rating >= 440N\n}\n";
    let p = parse(src);
    assert!(p.diags.is_empty(), "{:?}", p.diags);
    let pk = vec![PackageSrc {
        name: "acme.rover".to_owned(),
        deps: Vec::new(),
        modules: vec![ModuleSrc {
            path: vec!["rig".to_owned()],
            file: &p.file,
            enums: &[],
        }],
    }];
    let (resolver, errs) = Resolver::new(pk.clone());
    assert!(errs.is_empty(), "{errs:?}");
    let units = UnitTable::builtin();
    let ev = Evaluator::new(&resolver, &units, &pk).evaluate();
    assert!(ev.diagnostics.is_empty(), "{:?}", ev.diagnostics);
    assert_eq!(ev.checks.len(), 1);
    assert_eq!(ev.checks[0].verdict, Verdict::Holds);
}
