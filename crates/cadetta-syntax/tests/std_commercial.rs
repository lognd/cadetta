//! `std/src/commercial.cdt` carries the optional-member `Sourced` interface (stdlib.md 6).

// frob:ticket 01M4SWP82QQYMBZHRTMZA7KJ1K

use cadetta_syntax::ast::{FileEntry, InterfaceMemberKind, ItemKind};
use cadetta_syntax::parser::parse;

const SRC: &str = include_str!("../../../std/src/commercial.cdt");

// frob:tests crates/cadetta-syntax/tests/std_commercial.rs::sourced_members_are_all_optional kind=unit
#[test]
fn sourced_members_are_all_optional() {
    let parsed = parse(SRC);
    assert!(parsed.diags.is_empty(), "{:?}", parsed.diags);
    let ifaces: Vec<_> = parsed
        .file
        .entries
        .into_iter()
        .filter_map(|e| match e {
            FileEntry::Item(i) => match i.kind {
                ItemKind::Interface(x) => Some(x),
                _ => None,
            },
            _ => None,
        })
        .collect();
    assert_eq!(ifaces.len(), 1);
    assert_eq!(ifaces[0].name.name, "Sourced");
    let fields: Vec<(String, bool)> = ifaces[0]
        .members
        .iter()
        .map(|m| match &m.kind {
            InterfaceMemberKind::Field { name, optional, .. } => (name.name.clone(), *optional),
            other => panic!("unexpected member {other:?}"),
        })
        .collect();
    let names: Vec<&str> = fields.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(names, ["unit_cost", "lead_time", "moq", "supplier", "mass"]);
    assert!(fields.iter().all(|(_, o)| *o));
}
