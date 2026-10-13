//! `std/src/fits.cdt` carries the published ISO 286 figures (stdlib.md 1, 2).

// frob:ticket 01M4STANXPC2GK2PQN8E8NJG6J

use cadetta_syntax::ast::{FileEntry, ItemKind, Table};
use cadetta_syntax::parser::parse;

const SRC: &str = include_str!("../../../std/src/fits.cdt");

/// Parses the module and returns its tables, asserting no diagnostics.
fn tables() -> Vec<Table> {
    let parsed = parse(SRC);
    assert!(parsed.diags.is_empty(), "{:?}", parsed.diags);
    parsed
        .file
        .entries
        .into_iter()
        .filter_map(|e| match e {
            FileEntry::Item(i) => match i.kind {
                ItemKind::Table(t) => Some(t),
                _ => None,
            },
            _ => None,
        })
        .collect()
}

/// Reads one table's rows from the source text as (hi_mm, values in nm).
fn rows(table: &str) -> Vec<(i64, Vec<i64>)> {
    let header = format!("table {table} ");
    let mut lines = SRC.lines().skip_while(|l| !l.contains(&header));
    lines.next();
    lines
        .skip(2)
        .take_while(|l| !l.starts_with('}'))
        .map(|l| {
            let (k, v) = l.trim().split_once(" => ").expect("row");
            let hi = k.strip_suffix("mm").unwrap().parse().unwrap();
            let vals = v
                .split(", ")
                .map(|x| {
                    let um: f64 = x.strip_suffix("um").unwrap().parse().unwrap();
                    (um * 1000.0).round() as i64
                })
                .collect();
            (hi, vals)
        })
        .collect()
}

/// The row whose range (lo, hi] holds `size_mm`.
fn row(table: &str, size_mm: i64) -> Vec<i64> {
    rows(table)
        .into_iter()
        .find(|(hi, _)| size_mm <= *hi)
        .expect("in range")
        .1
}

// frob:tests crates/cadetta-syntax/tests/std_fits.rs::tables_parse_with_expected_shape kind=unit
#[test]
fn tables_parse_with_expected_shape() {
    let ts = tables();
    let shape: Vec<(String, usize, usize)> = ts
        .iter()
        .map(|t| (t.name.name.clone(), t.cols.len(), t.rows.len()))
        .collect();
    assert_eq!(
        shape,
        vec![
            ("ItGrade".to_owned(), 18, 13),
            ("ShaftUpper".to_owned(), 5, 13),
            ("ShaftUpperC".to_owned(), 1, 25),
            ("ShaftLower".to_owned(), 6, 13),
        ]
    );
    assert!(ts.iter().all(|t| t.citation.is_some()));
}

// frob:tests crates/cadetta-syntax/tests/std_fits.rs::deviations_match_published_iso286 kind=unit
#[test]
fn deviations_match_published_iso286() {
    // IT grades (ISO 286-1 table 1), um: 18-30 and 30-50 mm columns.
    assert_eq!(
        row("ItGrade", 20)[..8],
        [1_500, 2_500, 4_000, 6_000, 9_000, 13_000, 21_000, 33_000]
    );
    assert_eq!(
        row("ItGrade", 40)[..8],
        [1_500, 2_500, 4_000, 7_000, 11_000, 16_000, 25_000, 39_000]
    );
    assert_eq!(row("ItGrade", 500)[..4], [8_000, 10_000, 15_000, 20_000]);
    assert_eq!(row("ItGrade", 3)[0], 800);
    // Shaft d e f g h at 20 mm and 100 mm.
    assert_eq!(
        row("ShaftUpper", 20),
        [-65_000, -40_000, -20_000, -7_000, 0]
    );
    assert_eq!(
        row("ShaftUpper", 100),
        [-120_000, -72_000, -36_000, -12_000, 0]
    );
    // c uses the fine split: 10 mm is in (6, 10], 12 mm in (10, 14].
    assert_eq!(row("ShaftUpperC", 10), [-80_000]);
    assert_eq!(row("ShaftUpperC", 12), [-95_000]);
    assert_eq!(row("ShaftUpperC", 500), [-480_000]);
    // j5/6, j7, k4-7, m, n, p at 20 mm and 8 mm.
    assert_eq!(
        row("ShaftLower", 20),
        [-4_000, -8_000, 2_000, 8_000, 15_000, 22_000]
    );
    assert_eq!(
        row("ShaftLower", 8),
        [-2_000, -5_000, 1_000, 6_000, 10_000, 15_000]
    );
}

// frob:tests crates/cadetta-syntax/tests/std_fits.rs::bore_and_shaft_limits_resolve_to_expected_clearance kind=unit
#[test]
fn bore_and_shaft_limits_resolve_to_expected_clearance() {
    // 20 mm H7/g6: H7 = [0, +21], g6 = [es - IT6, es] = [-20, -7] um.
    let it = row("ItGrade", 20);
    let (it6, it7) = (it[5], it[6]);
    let es_g = row("ShaftUpper", 20)[3];
    let (hole_lo, hole_hi) = (0, it7);
    let (shaft_lo, shaft_hi) = (es_g - it6, es_g);
    assert_eq!(
        (hole_lo, hole_hi, shaft_lo, shaft_hi),
        (0, 21_000, -20_000, -7_000)
    );
    let (min_clr, max_clr) = (hole_lo - shaft_hi, hole_hi - shaft_lo);
    assert_eq!((min_clr, max_clr), (7_000, 41_000));
    // 20 mm H7/p6 is an interference fit: p6 = [ei, ei + IT6] = [22, 35] um.
    let ei_p = row("ShaftLower", 20)[5];
    let (min_clr, max_clr) = (hole_lo - (ei_p + it6), hole_hi - ei_p);
    assert_eq!((min_clr, max_clr), (-35_000, -1_000));
}
