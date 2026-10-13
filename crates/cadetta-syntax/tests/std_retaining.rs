//! `std/src/retaining.cdt` carries the Din471 and Din472 tables (stdlib.md 2).

// frob:ticket 01M4STAPK7FN8DM11MV7GWXE3Z

use cadetta_syntax::ast::{FileEntry, ItemKind};
use cadetta_syntax::parser::parse;

const SRC: &str = include_str!("../../../std/src/retaining.cdt");

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

/// Data lines of the named table as (key, cells).
fn rows(table: &str) -> Vec<(String, Vec<String>)> {
    let start = SRC.find(&format!("pub table {table} ")).expect("table");
    SRC[start..]
        .lines()
        .skip(3)
        .take_while(|l| l.trim() != "}")
        .map(|l| {
            let (k, v) = l.split_once("=>").expect("row");
            (
                k.trim().to_owned(),
                v.split(',').map(|c| c.trim().to_owned()).collect(),
            )
        })
        .collect()
}

// frob:tests crates/cadetta-syntax/tests/std_retaining.rs::both_tables_parse_cited_with_four_columns kind=unit
#[test]
fn both_tables_parse_cited_with_four_columns() {
    let t = tables();
    assert_eq!(t.len(), 2);
    assert_eq!((t[0].0.as_str(), t[0].1, t[0].3), ("Din471", 4, true));
    assert_eq!((t[1].0.as_str(), t[1].1, t[1].3), ("Din472", 4, true));
    for (name, _, n, _) in &t {
        assert_eq!(rows(name).len(), *n);
        assert!(rows(name).iter().all(|(_, c)| c.len() == 4));
    }
}

// frob:tests crates/cadetta-syntax/tests/std_retaining.rs::groove_tolerance_classes_and_golden_row_lookup kind=unit
#[test]
fn groove_tolerance_classes_and_golden_row_lookup() {
    for (_, c) in rows("Din472") {
        assert!(c[0].ends_with(" H11") && c[1].ends_with(" H13"), "{c:?}");
    }
    for (_, c) in rows("Din471") {
        assert!(c[0].ends_with(" h10") && c[1].ends_with(" H13"), "{c:?}");
    }
    let r = rows("Din472");
    let c = &r.iter().find(|(k, _)| k == "22mm").expect("22mm").1;
    assert_eq!(c[1], "1.1mm H13");
    assert_eq!(c[2], "1mm");
    assert_eq!(c[3], "1.5mm");
}
