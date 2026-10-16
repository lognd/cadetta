//! `std/src/lib.cdt` exports one `pub use` per standard (stdlib.md intro).

// frob:ticket 01M4SXVA290E2HY5B284N1B3H9

use cadetta_syntax::ast::FileEntry;
use cadetta_syntax::parser::parse;

const SRC: &str = include_str!("../../../std/src/lib.cdt");

/// The root parses clean and every entry is a `pub use` re-export (draft 1.5).
// frob:ticket 01M4VB62VJQV9AKSSQME8NWW6Y
#[test]
fn root_parses_clean_and_only_reexports() {
    let parsed = parse(SRC);
    assert!(parsed.diags.is_empty(), "{:?}", parsed.diags);
    assert_eq!(parsed.file.uses().count(), 14);
    assert_eq!(
        parsed
            .file
            .entries
            .iter()
            .filter(|e| matches!(e, FileEntry::Item(_)))
            .count(),
        0
    );
    assert!(
        parsed
            .file
            .uses()
            .all(|u| u.vis != cadetta_syntax::ast::Vis::Private)
    );
}

/// The named standards are all exported at the root.
// frob:ticket 01M4VB62VJQV9AKSSQME8NWW6Y
#[test]
fn root_exports_each_standard() {
    for n in [
        "iso273", "iso286", "din471", "din472", "iso724", "iso4762", "iso4032", "tapdrill",
    ] {
        assert!(SRC.contains(&format!("::{n}\n")), "{n}");
    }
}
