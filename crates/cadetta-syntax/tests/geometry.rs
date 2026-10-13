//! Conformance tests for `datum`, `on`, `in` and `place` syntax (geometry.md 2 to 4).

// frob:ticket 01M4SWFX0C2BXQGR8WQ2VN1G88

use cadetta_syntax::ast::{
    BinOp, ExprKind, FileEntry, InterfaceMemberKind, ItemKind, Member, MemberKind, TypeKind,
};
use cadetta_syntax::format::format_source;
use cadetta_syntax::parser::parse;

fn members(src: &str) -> Vec<Member> {
    let parsed = parse(src);
    assert!(parsed.diags.is_empty(), "{:?}", parsed.diags);
    for entry in parsed.file.entries {
        if let FileEntry::Item(item) = entry
            && let ItemKind::Component(c) = item.kind
        {
            return c.members;
        }
    }
    panic!("no component");
}

#[test]
fn datum_on_in_and_place_yield_nodes_with_spans() {
    // frob:tests crates/cadetta-syntax/src/geometry.rs::datum kind=unit
    let src = "assembly A {\n  datum top = origin.offset(z = height)\n  p: Point2 on top = Point2(1mm, 2mm)\n  q = p in D\n  place A at B\n}\n";
    let ms = members(src);
    let MemberKind::Datum(d) = &ms[0].kind else {
        panic!("datum")
    };
    assert_eq!(d.name.name, "top");
    assert_eq!(d.span.slice(src), "datum top = origin.offset(z = height)");
    assert!(matches!(
        d.value.as_ref().map(|v| &v.kind),
        Some(ExprKind::Call { .. })
    ));
    let MemberKind::Param(p) = &ms[1].kind else {
        panic!("param")
    };
    assert!(matches!(
        p.ty.as_ref().map(|t| &t.kind),
        Some(TypeKind::OnDatum { .. })
    ));
    let MemberKind::Param(q) = &ms[2].kind else {
        panic!("param")
    };
    assert!(matches!(&q.value.kind, ExprKind::Binary(BinOp::In, ..)));
    let MemberKind::Place(pl) = &ms[3].kind else {
        panic!("place")
    };
    assert_eq!(pl.span.slice(src), "place A at B");
    assert!(matches!(&pl.at.kind, ExprKind::Path(..)));
}

#[test]
fn place_targets_may_be_indexed_and_use_built_datums() {
    // frob:tests crates/cadetta-syntax/src/geometry.rs::place kind=unit
    let ms = members(
        "assembly A {\n  place rail.origin at origin\n  place feet.top at base.underside.offset(base.sites)\n  place feet[2].top at base.underside\n  place bracket.base at rail.top.offset(x = 40mm).rotate(z = 90deg)\n}\n",
    );
    assert!(ms.iter().all(|m| matches!(&m.kind, MemberKind::Place(_))));
    let MemberKind::Place(p) = &ms[1].kind else {
        unreachable!()
    };
    assert!(matches!(
        &p.target.kind,
        ExprKind::Path(..) | ExprKind::Field(..)
    ));
}

#[test]
fn old_place_spellings_are_e001_pointing_at_the_new_form() {
    // frob:tests crates/cadetta-syntax/src/geometry.rs::place kind=unit
    for (src, needle) in [
        ("assembly A {\n  place a.b on c\n}\n", "place a.b at c"),
        (
            "assembly A {\n  place a.b at c for i in 0..3\n}\n",
            "no `for` clause",
        ),
        ("assembly A {\n  place a.b c\n}\n", "expected `at`"),
    ] {
        let parsed = parse(src);
        assert!(
            parsed.diags.iter().any(|d| d.message.contains(needle)),
            "{src}: {:?}",
            parsed.diags
        );
    }
}

#[test]
fn at_stays_usable_as_an_argument_name() {
    // frob:tests crates/cadetta-syntax/src/geometry.rs::place kind=unit
    let parsed =
        parse("assembly A {\n  j = ScrewJoint(at = a.b, onto = c.d)\n  place a.b at c.d\n}\n");
    assert!(parsed.diags.is_empty(), "{:?}", parsed.diags);
}

#[test]
fn in_starts_a_bound_only_before_a_range() {
    // frob:tests crates/cadetta-syntax/src/parser.rs::parse kind=unit
    let ok = members("model M {\n  w in 1mm..2mm\n  check c for i in 0..N: i >= 0\n}\n");
    assert!(matches!(&ok[0].kind, MemberKind::Bound(_)));
    assert!(matches!(&ok[1].kind, MemberKind::Check(c) if c.each.is_some()));
    let bad = parse("model M {\n  w in D\n}\n");
    assert!(!bad.diags.is_empty());
}

#[test]
fn origin_is_reserved_as_a_member_name() {
    // frob:tests crates/cadetta-syntax/src/geometry.rs::reject_origin_name kind=unit
    for src in [
        "model M {\n  origin = 5mm\n}\n",
        "model M {\n  datum origin = origin\n}\n",
        "interface I {\n  origin: Length\n}\n",
    ] {
        let parsed = parse(src);
        assert!(
            parsed
                .diags
                .iter()
                .any(|d| d.message.contains("`origin` is reserved")),
            "{src}: {:?}",
            parsed.diags
        );
    }
}

#[test]
fn interface_datums_have_no_value() {
    // frob:tests crates/cadetta-syntax/src/geometry.rs::datum kind=unit
    let parsed = parse("interface I {\n  datum shaft\n  datum mount\n  shaft.z == mount.z\n}\n");
    assert!(parsed.diags.is_empty(), "{:?}", parsed.diags);
    let entry = parsed.file.entries.into_iter().next().expect("entry");
    let FileEntry::Item(item) = entry else {
        panic!("item")
    };
    let ItemKind::Interface(i) = item.kind else {
        panic!("interface")
    };
    assert!(matches!(&i.members[0].kind, InterfaceMemberKind::Datum(d) if d.value.is_none()));
    assert!(
        !parse("interface I {\n  datum a = origin\n}\n")
            .diags
            .is_empty()
    );
    assert!(!parse("model M {\n  datum a\n}\n").diags.is_empty());
}

#[test]
fn formatter_handles_datum_and_place() {
    // frob:tests crates/cadetta-syntax/src/format.rs::format_source kind=unit
    let src = "assembly Mount {\n  datum   top=origin.offset( z = height )\n  place rail.origin   at origin\n  place feet.top at base.top.offset(base.sites)\n}\ninterface I {\n datum shaft\n}\n";
    let once = format_source(src).expect("format");
    assert!(
        once.contains("  datum top = origin.offset(z = height)\n"),
        "{once}"
    );
    assert!(once.contains("  place rail.origin at origin\n"), "{once}");
    assert!(
        once.contains("  place feet.top at base.top.offset(base.sites)\n"),
        "{once}"
    );
    assert!(once.contains("  datum shaft\n"), "{once}");
    assert_eq!(format_source(&once).expect("again"), once);
}
