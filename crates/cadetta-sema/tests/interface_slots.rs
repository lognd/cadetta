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
    assert_eq!(
        q.nominal,
        Magnitude::new(Rational::new(2, 1).expect("denominator"), 0)
    );
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

// frob:ticket 01M4TRMJMNWCMHCE2H3BJF002X
// frob:tests with_selection
#[test]
fn selection_redirects_slot_reads_to_the_alternate() {
    use cadetta_sema::eval::Selection;
    use cadetta_sema::resolve::Ctx;
    use cadetta_syntax::ast::{Bind, ItemKind, MemberKind, SolverMemberKind};

    let src = "interface Motor { mass: Mass }\n\
               model MotorA { mass = 2kg }\n\
               model MotorB { mass = 5kg }\n\
               model Rig {\n  part motor: Motor = MotorA alt [MotorB]\n  \
               solver s {\n    script \"s.sh\"\n    in m = motor.mass\n    out w: Length\n  }\n}\n";
    let p = parse(src);
    assert!(p.diags.is_empty(), "{:?}", p.diags);
    let modules = vec!["rig".to_owned()];
    let pk = vec![PackageSrc {
        name: "acme.rover".to_owned(),
        deps: Vec::new(),
        modules: vec![ModuleSrc {
            path: modules.clone(),
            file: &p.file,
            enums: &[],
        }],
    }];
    let (resolver, errs) = Resolver::new(pk.clone());
    assert!(errs.is_empty(), "{errs:?}");
    let units = UnitTable::builtin();
    let expr = p
        .file
        .items()
        .find_map(|i| match &i.kind {
            ItemKind::Component(c) => c.members.iter().find_map(|m| match &m.kind {
                MemberKind::Solver(s) => s.members.iter().find_map(|m| match &m.kind {
                    SolverMemberKind::In(binds) => binds.first().map(|Bind { value, .. }| value),
                    _ => None,
                }),
                _ => None,
            }),
            _ => None,
        })
        .expect("solver input");
    let ctx = Ctx {
        package: "acme.rover".to_owned(),
        module: modules.clone(),
        component: Some("Rig".to_owned()),
        locals: Vec::new(),
    };
    let kg = |n| Magnitude::new(Rational::new(n, 1).expect("denominator"), 0);

    let mut primary = Evaluator::new(&resolver, &units, &pk);
    assert_eq!(
        primary
            .eval_input(ctx.clone(), expr)
            .expect("primary")
            .nominal,
        kg(2)
    );

    let sel = Selection {
        owner: CanonPath::item("acme.rover", &modules, "Rig"),
        slot: "motor".to_owned(),
        alt: CanonPath::item("acme.rover", &modules, "MotorB"),
    };
    assert_eq!(
        sel.key(),
        "acme.rover::rig::Rig/motor=acme.rover::rig::MotorB"
    );
    let mut swapped = Evaluator::new(&resolver, &units, &pk).with_selection(sel);
    assert_eq!(
        swapped.eval_input(ctx, expr).expect("alternate").nominal,
        kg(5)
    );
}
