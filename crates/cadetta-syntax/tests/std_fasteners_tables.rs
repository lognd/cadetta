//! `std/src/fasteners/tables.cdt` carries the metric fastener tables (stdlib.md 2).

// frob:ticket 01M4STAP388RZR0PTDBKBV6JM5

use cadetta_syntax::ast::{FileEntry, ItemKind};
use cadetta_syntax::parser::parse;

const SRC: &str = include_str!("../../../std/src/fasteners/tables.cdt");

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

/// Data lines of the named table as (key, cells); key-only rows have no cells.
fn rows(table: &str) -> Vec<(String, Vec<String>)> {
    let start = SRC.find(&format!("pub table {table} ")).expect("table");
    SRC[start..]
        .lines()
        .skip(if table == "Iso4762Len" { 2 } else { 3 })
        .take_while(|l| l.trim() != "}")
        .map(|l| match l.split_once("=>") {
            Some((k, v)) => (
                k.trim().to_owned(),
                v.split(',').map(|c| c.trim().to_owned()).collect(),
            ),
            None => (l.trim().to_owned(), vec![]),
        })
        .collect()
}

/// Cells of the named table's row for `MetricThread::<size>`.
fn row(table: &str, size: &str) -> Vec<String> {
    let key = format!("MetricThread::{size}");
    rows(table)
        .into_iter()
        .find(|(k, _)| *k == key)
        .expect("row")
        .1
}

// frob:tests crates/cadetta-syntax/tests/std_fasteners_tables.rs::eight_tables_parse_cited_with_spec_columns kind=unit
#[test]
fn eight_tables_parse_cited_with_spec_columns() {
    let t = tables();
    let want = [
        ("Thread", 4, 12),
        ("Iso273", 3, 12),
        ("TapDrill", 3, 12),
        ("Iso4762Head", 5, 12),
        ("Iso7380Head", 5, 8),
        ("Iso10642Head", 5, 9),
        ("Iso4762Len", 0, 36),
        ("Iso4032", 2, 12),
    ];
    assert_eq!(t.len(), want.len());
    for ((name, cols, n, cited), (wn, wc, wr)) in t.iter().zip(want) {
        assert_eq!((name.as_str(), *cols, *n), (wn, wc, wr));
        assert!(cited, "{name} uncited");
        assert_eq!(rows(name).len(), *n);
    }
    for (_, c) in rows("Thread") {
        assert_eq!(c.len(), 4);
    }
}

// frob:tests crates/cadetta-syntax/tests/std_fasteners_tables.rs::golden_assumed_values_match_the_tables kind=unit
#[test]
fn golden_assumed_values_match_the_tables() {
    // tests/golden/01-bracket/expected.toml header.
    assert_eq!(row("Iso273", "M4")[1], "4.5mm");
    assert_eq!(row("Iso273", "M5")[1], "5.5mm");
    assert_eq!(row("Thread", "M4")[0], "4mm");
    assert_eq!(row("Thread", "M5")[0], "5mm");
    let h4 = row("Iso4762Head", "M4");
    assert_eq!((h4[3].as_str(), h4[4].as_str()), ("8mm", "4.4mm"));
    let h5 = row("Iso4762Head", "M5");
    assert_eq!((h5[3].as_str(), h5[4].as_str()), ("9.5mm", "5.4mm"));
    assert_eq!(row("TapDrill", "M4")[0], "3.3mm");
    assert_eq!(row("TapDrill", "M5")[0], "4.2mm");
    let lens: Vec<String> = rows("Iso4762Len").into_iter().map(|(k, _)| k).collect();
    for want in ["8mm", "10mm", "12mm", "16mm"] {
        assert!(lens.iter().any(|l| l == want), "{want}");
    }
}

// frob:tests crates/cadetta-syntax/tests/std_fasteners_tables.rs::published_limits_are_kept_as_tolerances kind=unit
#[test]
fn published_limits_are_kept_as_tolerances() {
    for (_, c) in rows("Iso4762Head") {
        assert!(c[1].contains(" [+0 -") && c[1].ends_with("mm]"), "{c:?}");
        assert!(!c[0].contains('['));
    }
    for (_, c) in rows("Iso4032") {
        assert!(c[1].contains(" [+0 -"), "{c:?}");
    }
    assert_eq!(row("Iso4762Head", "M5")[1], "5mm [+0 -0.18mm]");
}

// frob:ticket 01M4VSSDQZJ13JHVPXB3CHC33P

// frob:tests crates/cadetta-syntax/tests/std_fasteners_tables.rs::button_and_flat_head_pockets_follow_their_standards kind=unit
#[test]
fn button_and_flat_head_pockets_follow_their_standards() {
    // ISO 7380 covers M3 to M16 only; ISO 10642 covers M3 to M20.
    let button: Vec<String> = rows("Iso7380Head").into_iter().map(|(k, _)| k).collect();
    assert!(button.contains(&"MetricThread::M16".to_owned()));
    assert!(!button.contains(&"MetricThread::M20".to_owned()));
    assert!(!button.contains(&"MetricThread::M2".to_owned()));
    assert_eq!(rows("Iso10642Head").len(), 9);
    // ISO 7380 M6: dk 10.5, k 3.3; derived pocket 11.5 x 3.8.
    let b6 = row("Iso7380Head", "M6");
    assert_eq!((b6[0].as_str(), b6[1].as_str()), ("10.5mm", "3.3mm"));
    assert_eq!((b6[3].as_str(), b6[4].as_str()), ("11.5mm", "3.8mm"));
    // ISO 10642 pocket is the countersink: head diameter by head height, flush.
    for (key, c) in rows("Iso10642Head") {
        assert_eq!(
            (c[3].as_str(), c[4].as_str()),
            (c[0].as_str(), c[1].as_str()),
            "{key}"
        );
    }
    assert_eq!(row("Iso10642Head", "M8")[0], "17.92mm");
    // Every pocket is at least as large as its head.
    for t in ["Iso7380Head", "Iso10642Head"] {
        for (key, c) in rows(t) {
            let n = |s: &str| s.trim_end_matches("mm").parse::<f64>().unwrap();
            assert!(n(&c[3]) >= n(&c[0]) && n(&c[4]) >= n(&c[1]), "{t} {key}");
        }
    }
}
