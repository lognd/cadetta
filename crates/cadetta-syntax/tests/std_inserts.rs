//! `std/src/inserts.cdt` carries the WireInsertInch and HeatSet tables (stdlib.md 2).

// frob:ticket 01M51461751SB8Z9SVSAAVM8QV

use cadetta_syntax::ast::{FileEntry, ItemKind};
use cadetta_syntax::parser::parse;

const SRC: &str = include_str!("../../../std/src/inserts.cdt");

/// Parses the module, asserting no diagnostics, and returns (name, cols, rows, cited) per table.
fn tables() -> Vec<(String, usize, usize, bool)> {
    let parsed = parse(SRC);
    assert!(parsed.diags.is_empty(), "{:?}", parsed.diags);
    parsed
        .file
        .entries
        .into_iter()
        .filter_map(|e| match e {
            FileEntry::Item(i) => match i.kind {
                ItemKind::Table(t) => Some((
                    t.name.name.clone(),
                    t.cols.len(),
                    t.rows.len(),
                    t.citation.is_some(),
                )),
                _ => None,
            },
            _ => None,
        })
        .collect()
}

// frob:tests crates/cadetta-syntax/tests/std_inserts.rs::inserts_tables_are_cited_with_expected_shape kind=unit
#[test]
fn inserts_tables_are_cited_with_expected_shape() {
    let t = tables();
    assert_eq!(
        t,
        vec![
            ("WireInsertInch".to_owned(), 5, 16, true),
            ("HeatSet".to_owned(), 4, 7, true),
        ]
    );
}
