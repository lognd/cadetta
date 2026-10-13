//! `std/src/fasteners/policy.cdt` carries FastenerPolicy and IsoDefault (stdlib.md 4).

// frob:ticket 01M4STAP93AYTZDJYWCCAS1TRH

use cadetta_syntax::ast::{Component, FileEntry, Interface, ItemKind};
use cadetta_syntax::parser::parse;

const SRC: &str = include_str!("../../../std/src/fasteners/policy.cdt");
const TABLES: &str = include_str!("../../../std/src/fasteners/tables.cdt");
const MATERIALS: &str = include_str!("../../../std/src/materials.cdt");

/// Parses the module, asserting no diagnostics, and returns its interfaces and components.
fn items() -> (Vec<Interface>, Vec<Component>) {
    let parsed = parse(SRC);
    assert!(parsed.diags.is_empty(), "{:?}", parsed.diags);
    let (mut ifaces, mut comps) = (vec![], vec![]);
    for e in parsed.file.entries {
        if let FileEntry::Item(i) = e {
            match i.kind {
                ItemKind::Interface(x) => ifaces.push(x),
                ItemKind::Component(x) => comps.push(x),
                _ => {}
            }
        }
    }
    (ifaces, comps)
}

/// Returns the row lines of the `pub table <name>` block in `src`.
fn rows<'a>(src: &'a str, name: &str) -> Vec<&'a str> {
    let start = src.find(&format!("pub table {name}")).expect("table");
    src[start..]
        .lines()
        .skip(1)
        .take_while(|l| l.trim() != "}")
        .map(str::trim)
        .filter(|l| !l.starts_with("key ") && !l.starts_with("cols "))
        .collect()
}

// frob:tests crates/cadetta-syntax/tests/std_fasteners_policy.rs::iso_default_implements_policy kind=unit
#[test]
fn iso_default_implements_policy() {
    let (ifaces, comps) = items();
    assert_eq!(ifaces.len(), 1);
    assert_eq!(ifaces[0].name.name, "FastenerPolicy");
    assert_eq!(ifaces[0].members.len(), 7);
    assert_eq!(comps.len(), 1);
    assert_eq!(comps[0].name.name, "IsoDefault");
    assert!(format!("{:?}", comps[0].implements[0]).contains("FastenerPolicy"));
    assert!(comps[0].citation.is_some());
}

// frob:tests crates/cadetta-syntax/tests/std_fasteners_policy.rs::policy_tables_are_typed kind=unit
#[test]
fn policy_tables_are_typed() {
    assert!(SRC.contains("cols factor: Ratio"));
    assert!(SRC.contains("cols cut: TapColumn, form: TapColumn"));
    assert!(SRC.contains("pub enum TapColumn { Cut75, Cut65, Form }"));
    assert!(SRC.contains("TapClass::Soft => TapColumn::Cut75, TapColumn::Form"));
    assert!(SRC.contains("TapClass::Hard => TapColumn::Cut65, TapColumn::Form"));
}

// frob:tests crates/cadetta-syntax/tests/std_fasteners_policy.rs::iso_default_values_and_sources kind=unit
#[test]
fn iso_default_values_and_sources() {
    for line in [
        "lengths          = Iso4762Len",
        "bottom_clearance = 1mm",
        "tap_lead_in      = 3",
        "drill_point      = 0.3",
    ] {
        assert!(SRC.contains(line), "{line}");
    }
    // Engagement factors repeat Materials.engagement_factor row for row.
    let eng = rows(SRC, "IsoEngagement");
    let mat = rows(MATERIALS, "Materials");
    assert_eq!(eng.len(), mat.len());
    for (e, m) in eng.iter().zip(&mat) {
        let (ek, ev) = e.split_once("=>").unwrap();
        let (mk, mv) = m.split_once("=>").unwrap();
        assert_eq!(ek.trim(), mk.trim());
        assert_eq!(ev.trim(), mv.rsplit(',').next().unwrap().trim());
    }
    // Every shelf drill is a TapDrill size, and every TapDrill size is on the shelf.
    let shelf: Vec<String> = rows(SRC, "IsoDrills")
        .iter()
        .map(|s| s.to_string())
        .collect();
    for r in rows(TABLES, "TapDrill") {
        for v in r.split_once("=>").unwrap().1.split(',') {
            assert!(shelf.contains(&v.trim().to_string()), "{v}");
        }
    }
}

// frob:tests crates/cadetta-syntax/tests/std_fasteners_policy.rs::default_is_std_supplied_and_unlinted kind=unit
#[test]
fn default_is_std_supplied_and_unlinted() {
    // std supplies one pub default under the stdlib.md 4 name; the file declares no
    // `#![default]` itself, so nothing here overrides a project [defaults] entry.
    assert!(SRC.contains("pub ots IsoDefault: FastenerPolicy"));
    assert!(
        SRC.lines()
            .filter(|l| !l.starts_with("//"))
            .all(|l| !l.contains("#![default"))
    );
}
