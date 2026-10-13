//! Optional parameters `name?: Type` parse, format and round-trip (language.md 3.3).

// frob:ticket 01M4STAPFPQ9GV8XJB4XCK8EGP

use cadetta_syntax::ast::{FileEntry, ItemKind};
use cadetta_syntax::format::format_source;
use cadetta_syntax::parser::parse;

const SRC: &str = "assembly C(a: Length, b?: Length, c: Count = 1) {\n}\n";

// frob:tests crates/cadetta-syntax/tests/optional_params.rs::optional_marker_parses kind=unit
#[test]
fn optional_marker_parses() {
    let parsed = parse(SRC);
    assert!(parsed.diags.is_empty(), "{:?}", parsed.diags);
    let flags: Vec<bool> = parsed
        .file
        .entries
        .iter()
        .find_map(|e| match e {
            FileEntry::Item(i) => match &i.kind {
                ItemKind::Component(c) => Some(c.params.iter().map(|p| p.optional).collect()),
                _ => None,
            },
            _ => None,
        })
        .expect("component");
    assert_eq!(flags, [false, true, false]);
}

// frob:tests crates/cadetta-syntax/tests/optional_params.rs::optional_marker_round_trips kind=unit
#[test]
fn optional_marker_round_trips() {
    let once = format_source(SRC).expect("format");
    assert!(once.contains("b?: Length"), "{once}");
    assert_eq!(format_source(&once).expect("format"), once);
}
