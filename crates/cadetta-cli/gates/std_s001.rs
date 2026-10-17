//! frob:ticket 01M4WE56Z5R66XFD87JQ07FCB2
//! Permanent S001 gate: a copy of the `std` tree, renamed `stdgate` (the name `std` is reserved
//! and std findings are filtered by `is_std`), is analysed as an ordinary in-scope package and
//! must carry no uncited `ots` value or table.

use std::fs;
use std::path::{Path, PathBuf};

use cadetta::pipeline::analyze;
use cadetta::project::Project;

fn std_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../std")
}

/// Copy `from` into `to` recursively.
fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).expect("mkdir");
    for entry in fs::read_dir(from).expect("read_dir") {
        let entry = entry.expect("entry");
        let dest = to.join(entry.file_name());
        if entry.file_type().expect("type").is_dir() {
            copy_tree(&entry.path(), &dest);
        } else {
            fs::copy(entry.path(), dest).expect("copy");
        }
    }
}

/// Copy `std/` to a temp dir as package `stdgate`, applying `edit` to `src/lib.cdt`; returns
/// the S001 diagnostics of analysing that copy.
fn s001_of_copy(tag: &str, edit: impl FnOnce(String) -> String) -> Vec<String> {
    let tmp = std::env::temp_dir().join(format!("cadetta-std-s001-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp);
    copy_tree(&std_dir(), &tmp);
    let manifest = tmp.join("cadetta.toml");
    let text = fs::read_to_string(&manifest).expect("manifest");
    fs::write(
        &manifest,
        text.replace("name = \"std\"", "name = \"stdgate\""),
    )
    .expect("write");
    let lib = tmp.join("src/lib.cdt");
    let text = fs::read_to_string(&lib).expect("lib.cdt");
    fs::write(&lib, edit(text)).expect("write");
    let (project, scope) = Project::open(Some(&tmp), None).expect("std copy opens");
    let analysis = analyze(&project, &scope);
    let _ = fs::remove_dir_all(&tmp);
    assert!(!analysis.halted, "std must parse");
    analysis
        .diagnostics
        .iter()
        .filter(|d| d.code == "S001")
        .map(|d| format!("{} {}", d.code, d.message))
        .collect()
}

// frob:tests crates/cadetta-cli/src/pipeline.rs::analyze kind=integration
#[test]
fn std_tree_has_no_s001() {
    let found = s001_of_copy("clean", |t| t);
    assert!(found.is_empty(), "uncited std values: {found:#?}");
}

// frob:tests crates/cadetta-cli/src/pipeline.rs::analyze kind=integration
#[test]
fn uncited_ots_value_in_std_src_fails_the_gate() {
    let found = s001_of_copy("dirty", |mut t| {
        t.push_str("\nots Uncited {\n  datum top = origin.offset(z = 5mm)\n}\n");
        t
    });
    assert!(
        !found.is_empty(),
        "the gate must catch an uncited ots value"
    );
}
