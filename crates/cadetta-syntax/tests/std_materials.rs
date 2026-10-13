//! `std/src/materials.cdt` carries the 14 Material variants and the Materials table (stdlib.md 2, 3).

// frob:ticket 01M4STAP0CCQTR0MM7TAMWC9E8

use cadetta_syntax::ast::{FileEntry, ItemKind};
use cadetta_syntax::parser::parse;

const SRC: &str = include_str!("../../../std/src/materials.cdt");

const VARIANTS: [&str; 14] = [
    "Al6061T6",
    "Al7075T6",
    "Al5052H32",
    "Ss304",
    "Ss316",
    "Steel1018",
    "Steel4140",
    "CastIron",
    "Brass360",
    "Petg",
    "Pla",
    "Abs",
    "Nylon6",
    "Pom",
];

/// Parses the module, asserting no diagnostics, and returns its items.
fn items() -> Vec<ItemKind> {
    let parsed = parse(SRC);
    assert!(parsed.diags.is_empty(), "{:?}", parsed.diags);
    parsed
        .file
        .entries
        .into_iter()
        .filter_map(|e| match e {
            FileEntry::Item(i) => Some(i.kind),
            _ => None,
        })
        .collect()
}

/// The table's data lines: `Material::X => cells...`, as (variant, cells).
fn rows() -> Vec<(String, Vec<String>)> {
    SRC.lines()
        .filter_map(|l| l.trim().strip_prefix("Material::"))
        .map(|l| {
            let (k, v) = l.split_once("=>").expect("row");
            (
                k.trim().to_owned(),
                v.split(',').map(|c| c.trim().to_owned()).collect(),
            )
        })
        .collect()
}

// frob:tests crates/cadetta-syntax/tests/std_materials.rs::every_variant_has_properties_and_a_citation kind=unit
#[test]
fn every_variant_has_properties_and_a_citation() {
    let kinds = items();
    let tables: Vec<_> = kinds
        .iter()
        .filter_map(|k| match k {
            ItemKind::Table(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(tables.len(), 1);
    assert_eq!(tables[0].name.name, "Materials");
    assert_eq!(tables[0].cols.len(), 4);
    assert_eq!(tables[0].rows.len(), 14);
    assert!(tables[0].citation.is_some());
    let r = rows();
    let keys: Vec<&str> = r.iter().map(|(k, _)| k.as_str()).collect();
    assert_eq!(keys, VARIANTS);
    assert!(r.iter().all(|(_, cells)| cells.len() == 4));
}

// frob:tests crates/cadetta-syntax/tests/std_materials.rs::lookup_by_key_returns_matching_row kind=unit
#[test]
fn lookup_by_key_returns_matching_row() {
    let r = rows();
    let get = |k: &str| &r.iter().find(|(n, _)| n == k).expect("row").1;
    assert_eq!(
        get("Al6061T6"),
        &["2.70g/cm^3", "276MPa", "TapClass::Soft", "2.0"]
    );
    assert_eq!(
        get("Ss304"),
        &["8.00g/cm^3", "215MPa", "TapClass::Hard", "1.0"]
    );
    assert_eq!(get("Brass360")[3], "1.5");
    assert_eq!(get("CastIron")[3], "1.5");
}
