//! ScrewJoint arguments and named checks in `std/src/fasteners/screwjoint.cdt` (stdlib.md 3).

// frob:ticket 01M4STAPFPQ9GV8XJB4XCK8EGP

use cadetta_syntax::ast::{Component, FileEntry, ItemKind, MemberKind};
use cadetta_syntax::parser::parse;

const SRC: &str = include_str!("../../../std/src/fasteners/screwjoint.cdt");

/// Parses the module, asserting no diagnostics, and returns the ScrewJoint component.
fn joint() -> Component {
    let parsed = parse(SRC);
    assert!(parsed.diags.is_empty(), "{:?}", parsed.diags);
    for e in parsed.file.entries {
        if let FileEntry::Item(i) = e
            && let ItemKind::Component(c) = i.kind
            && c.name.name == "ScrewJoint"
        {
            return c;
        }
    }
    panic!("no ScrewJoint");
}

/// The attribute names on the member that declares check `name`.
fn check_attrs(c: &Component, name: &str) -> Option<Vec<String>> {
    c.members.iter().find_map(|m| match &m.kind {
        MemberKind::Check(k) if k.name.name == name => {
            Some(m.attrs.iter().map(|a| a.name.name.clone()).collect())
        }
        _ => None,
    })
}

// frob:tests crates/cadetta-syntax/tests/std_fasteners_screwjoint_checks.rs::arguments_match_the_spec kind=unit
#[test]
fn arguments_match_the_spec() {
    let c = joint();
    let find = |n: &str| {
        c.params
            .iter()
            .find(|p| p.name.name == n)
            .unwrap_or_else(|| panic!("{n}"))
    };
    // Required: no default, not optional.
    for n in ["thread", "clamp", "policy"] {
        let p = find(n);
        assert!(!p.optional && p.default.is_none(), "{n}");
    }
    // Optional, no neutral value: absent when omitted.
    for n in [
        "into",
        "host",
        "count",
        "depth_available",
        "screw_cost",
        "at",
        "onto",
    ] {
        assert!(find(n).optional, "{n}");
    }
    // Defaulted.
    for n in ["head", "clearance", "counterbore", "tap"] {
        let p = find(n);
        assert!(!p.optional && p.default.is_some(), "{n}");
    }
    for v in [
        "TapKind::Vendor",
        "TapKind::Form",
        "HeadKind::FlatHead",
        "HeadKind::ButtonHead",
    ] {
        let _ = v;
    }
    assert!(SRC.contains("pub enum TapKind { Cut, Form, Vendor }"));
    assert!(SRC.contains("pub enum HeadKind { SocketHead, ButtonHead, FlatHead }"));
    assert!(SRC.contains("priv material = if given(into) { into } else { host.material }"));
}

// frob:tests crates/cadetta-syntax/tests/std_fasteners_screwjoint_checks.rs::named_checks_and_their_presence_rules kind=unit
#[test]
fn named_checks_and_their_presence_rules() {
    let c = joint();
    let names: Vec<String> = c
        .members
        .iter()
        .filter_map(|m| match &m.kind {
            MemberKind::Check(k) => Some(k.name.name.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(
        names,
        [
            "engaged",
            "not_bottomed",
            "grip_positive",
            "depth_ok",
            "aligned"
        ]
    );
    // Always present: no `when`.
    for n in ["engaged", "not_bottomed", "grip_positive"] {
        assert_eq!(check_attrs(&c, n), Some(vec![]), "{n}");
    }
    // Conditional: depth_ok with depth_available, aligned with at and onto.
    for n in ["depth_ok", "aligned"] {
        assert_eq!(check_attrs(&c, n), Some(vec!["when".to_owned()]), "{n}");
    }
    for line in [
        "check engaged: engagement >= min_engagement",
        "check not_bottomed: thread_depth > screw_length - grip",
        "check grip_positive: grip > 0mm",
        "check depth_ok: depth_needed <= depth_available",
        "#[when(given(depth_available))]",
        "#[when(given(at) and given(onto))]",
        "priv depth_needed = if tap == TapKind::Vendor { thread_depth } else { drill_depth }",
    ] {
        assert!(SRC.contains(line), "{line}");
    }
}

// frob:tests crates/cadetta-syntax/tests/std_fasteners_screwjoint_checks.rs::vendor_drops_the_drill_members_and_needs_depth kind=unit
#[test]
fn vendor_drops_the_drill_members_and_needs_depth() {
    let c = joint();
    for n in ["tap_drill", "drill_depth", "column", "drill_nominal"] {
        let m = c
            .members
            .iter()
            .find(|m| match &m.kind {
                MemberKind::Param(p) => p.name.name == n,
                MemberKind::Let(l) => l.name.name == n,
                _ => false,
            })
            .unwrap_or_else(|| panic!("{n}"));
        assert!(m.attrs.iter().any(|a| a.name.name == "when"), "{n}");
    }
    assert!(SRC.contains("#![requires(depth_available, when = tap == TapKind::Vendor)]"));
    assert!(SRC.contains("fmt(\"{} - 6H, {} DP\", thread, thread_depth)"));
}
