//! Every golden package source file parses without syntax errors (tests/golden/README.md).

// frob:ticket 01M4TA3CNEZ76JC3DHHNANZB20

use std::path::{Path, PathBuf};

use cadetta_syntax::parser::parse;

fn cdt_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .expect("read_dir")
        .map(|e| e.expect("entry").path())
        .collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            cdt_files(&p, out);
        } else if p.extension().is_some_and(|e| e == "cdt") {
            out.push(p);
        }
    }
}

#[test]
fn golden_sources_parse_without_syntax_errors() {
    // frob:tests crates/cadetta-syntax/src/parser.rs::parse kind=unit
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden");
    let mut files = Vec::new();
    cdt_files(&root, &mut files);
    assert!(!files.is_empty(), "no golden .cdt files found");
    let mut failures = Vec::new();
    for f in &files {
        let src = std::fs::read_to_string(f).expect("read");
        let parsed = parse(&src);
        for d in &parsed.diags {
            let line = src[..d.span.start.min(src.len())].matches('\n').count() + 1;
            let name = f.strip_prefix(&root).unwrap_or(f).display();
            failures.push(format!("{name}:{line}: {} {}", d.code.code(), d.message));
        }
    }
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

fn clean(src: &str) -> cadetta_syntax::parser::Parsed {
    let parsed = parse(src);
    assert!(parsed.diags.is_empty(), "{:?}", parsed.diags);
    parsed
}

/// Each grammar fix the goldens needed: parses clean and re-formats to a stable form.
fn assert_stable(src: &str) {
    clean(src);
    let once = cadetta_syntax::format::format_source(src).expect("format");
    let twice = cadetta_syntax::format::format_source(&once).expect("format again");
    assert_eq!(once, twice, "formatter is not idempotent");
    clean(&once);
}

#[test]
fn citation_after_bound_check_and_fit_statements() {
    // frob:tests crates/cadetta-syntax/src/parser.rs::member kind=unit
    assert_stable(
        "ots O {\n  a >= 1mm @s:\"x\"\n  b in 1mm..2mm @s\n  check c: a > 0mm @assumed:\"y\"\n  fit f = p6 on a @s\n  d = 1mm @s #[role(hole)]\n}\n",
    );
}

#[test]
fn citation_after_named_constructor_argument() {
    // frob:tests crates/cadetta-syntax/src/parser.rs::args kind=unit
    assert_stable("assembly A {\n  part p: J(cost = 0.12USD @fast, n = 2)\n}\n");
}

#[test]
fn path_pair_fit_may_omit_on() {
    // frob:tests crates/cadetta-syntax/src/parser.rs::fit kind=unit
    assert_stable("assembly A {\n  fit f = a.b / c.d [-0.4mm .. -0.1mm]\n}\n");
    let bad = parse("assembly A {\n  fit f = H7/p6\n}\n");
    assert!(!bad.diags.is_empty(), "class fits still need `on`");
}

#[test]
fn unit_words_in_attribute_arguments() {
    // frob:tests crates/cadetta-syntax/src/parser.rs::attr kind=unit
    assert_stable("model M {\n  #![display(in/32)]\n  x = 1 #[display(%)]\n}\n");
}

#[test]
fn leading_operator_continues_the_statement() {
    // frob:tests crates/cadetta-syntax/src/lexer.rs::run kind=unit
    assert_stable("model M {\n  let a = 1mm + 2mm\n          + 3mm\n          * 2\n}\n");
    let two = parse("model M {\n  let a = 1mm\n  /// doc\n  b = 2mm\n}\n");
    assert!(two.diags.is_empty(), "{:?}", two.diags);
}

#[test]
fn solver_binds_continue_after_a_comma_and_allow_checks() {
    // frob:tests crates/cadetta-syntax/src/parser_items.rs::solver kind=unit
    assert_stable(
        "solver s @\"s.py\" {\n  in a = 1mm,\n      b = 2mm\n  out o: Length\n  deps \"x.csv\"\n  check pos: o > 0mm\n}\n",
    );
}
