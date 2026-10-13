//! `std/src/fasteners/screw.cdt` carries the Screw interface and the Iso4762 family (stdlib.md 2).

// frob:ticket 01M4STAP69F2SZMD1C8HBVSAZC

use cadetta_syntax::ast::{Component, FileEntry, Interface, ItemKind};
use cadetta_syntax::parser::parse;

const SRC: &str = include_str!("../../../std/src/fasteners/screw.cdt");
const TABLES: &str = include_str!("../../../std/src/fasteners/tables.cdt");

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

// frob:tests crates/cadetta-syntax/tests/std_fasteners_screw.rs::iso4762_implements_screw kind=unit
#[test]
fn iso4762_implements_screw() {
    let (ifaces, comps) = items();
    assert_eq!(ifaces.len(), 1);
    assert_eq!(ifaces[0].name.name, "Screw");
    assert_eq!(comps.len(), 1);
    let c = &comps[0];
    assert_eq!(c.name.name, "Iso4762");
    assert_eq!(c.implements.len(), 1);
    assert!(format!("{:?}", c.implements[0]).contains("Screw"));
    let params: Vec<&str> = c.params.iter().map(|p| p.name.name.as_str()).collect();
    assert_eq!(params, ["thread", "length"]);
    assert!(c.citation.is_some());
}

// frob:tests crates/cadetta-syntax/tests/std_fasteners_screw.rs::iso4762_accepts_any_length_but_len_table_is_exact kind=unit
#[test]
fn iso4762_accepts_any_length_but_len_table_is_exact() {
    // The family never reads Iso4762Len, so any length is accepted ...
    assert!(!SRC.contains("Iso4762Len["));
    // ... while a direct Iso4762Len[x] lookup misses (E007) off the preferred list.
    let start = TABLES.find("pub table Iso4762Len").expect("table");
    let body: Vec<&str> = TABLES[start..]
        .lines()
        .skip(2)
        .take_while(|l| l.trim() != "}")
        .map(str::trim)
        .collect();
    assert!(body.contains(&"10mm"));
    assert!(!body.contains(&"9mm"));
    assert!(body.iter().all(|l| !l.contains("=>")));
}
