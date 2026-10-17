//! `std/src/fasteners/inch.cdt` carries the inch thread tables, each cited, and std is 0.2.0 (stdlib.md 1, 2).

// frob:ticket 01M4STAPTT2HJK6J5Y2HP6PTJH

use cadetta_syntax::parser::parse;

const SRC: &str = include_str!("../../../std/src/fasteners/inch.cdt");
const MANIFEST: &str = include_str!("../../../std/cadetta.toml");

/// Returns the row lines of the `pub table <name>` block.
fn rows<'a>(name: &str) -> Vec<&'a str> {
    let start = SRC.find(&format!("pub table {name}")).expect("table");
    SRC[start..]
        .lines()
        .skip(1)
        .take_while(|l| l.trim() != "}")
        .map(str::trim)
        .filter(|l| !l.starts_with("key ") && !l.starts_with("cols "))
        .collect()
}

// frob:tests crates/cadetta-syntax/tests/std_fasteners_inch.rs::inch_module_parses_and_tables_are_cited kind=unit
#[test]
fn inch_module_parses_and_tables_are_cited() {
    let parsed = parse(SRC);
    assert!(parsed.diags.is_empty(), "{:?}", parsed.diags);
    for t in [
        "InchThreadData",
        "InchTapDrill",
        "InchClearance",
        "InchSocketHead",
        "InchLen",
    ] {
        let line = SRC
            .lines()
            .find(|l| l.starts_with(&format!("pub table {t} ")))
            .unwrap_or_else(|| panic!("{t}"));
        assert!(line.contains(" @"), "{t} must carry a citation: {line}");
    }
}

// frob:tests crates/cadetta-syntax/tests/std_fasteners_inch.rs::every_thread_has_a_row_in_each_table kind=unit
#[test]
fn every_thread_has_a_row_in_each_table() {
    for t in [
        "InchThreadData",
        "InchTapDrill",
        "InchClearance",
        "InchSocketHead",
    ] {
        assert_eq!(rows(t).len(), 16, "{t}");
    }
}

// frob:tests crates/cadetta-syntax/tests/std_fasteners_inch.rs::std_version_is_0_2_0 kind=unit
#[test]
fn std_version_is_0_2_0() {
    assert!(MANIFEST.contains("version = \"0.2.0\""));
}
