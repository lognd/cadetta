//! `std/src/lib.cdt` exports one `pub cite` per standard (stdlib.md intro).

// frob:ticket 01M4SXVA290E2HY5B284N1B3H9

use cadetta_syntax::ast::{FileEntry, ItemKind};
use cadetta_syntax::parser::parse;

const SRC: &str = include_str!("../../../std/src/lib.cdt");

/// The root parses clean and every item is a cite.
#[test]
fn root_parses_clean_and_only_cites() {
    let parsed = parse(SRC);
    assert!(parsed.diags.is_empty(), "{:?}", parsed.diags);
    let items: Vec<_> = parsed
        .file
        .entries
        .iter()
        .filter_map(|e| match e {
            FileEntry::Item(i) => Some(i),
            _ => None,
        })
        .collect();
    assert_eq!(items.len(), 8);
    assert!(items.iter().all(|i| matches!(i.kind, ItemKind::Cite(_))));
}

/// The named standards are all exported at the root.
#[test]
fn root_exports_each_standard() {
    for n in [
        "iso273", "iso286", "din471", "din472", "iso724", "iso4762", "iso4032", "tapdrill",
    ] {
        assert!(SRC.contains(&format!("pub cite {n} = ")), "{n}");
    }
}
