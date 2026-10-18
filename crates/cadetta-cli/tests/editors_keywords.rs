//! Keeps the editor grammars' keyword list in step with the lexer.

use std::collections::BTreeSet;
use std::path::Path;

use cadetta_syntax::lexer::KEYWORDS;

/// Collect every keyword in `editors/grammar/cadetta.toml`, in file order.
fn grammar_keywords() -> Vec<String> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../editors/grammar/cadetta.toml");
    let text = std::fs::read_to_string(&path).expect("read editors/grammar/cadetta.toml");
    let doc: toml::Table = text.parse().expect("parse editors/grammar/cadetta.toml");
    let groups = doc
        .get("keywords")
        .and_then(|v| v.as_table())
        .expect("[keywords] table");
    let mut out = Vec::new();
    for (group, list) in groups {
        let list = list
            .as_array()
            .unwrap_or_else(|| panic!("[keywords].{group} must be an array"));
        for word in list {
            out.push(word.as_str().expect("keyword string").to_string());
        }
    }
    out
}

// frob:ticket 01M4VHFFWS5EWZW087G98FFJ8V
/// The editor grammar's reserved words equal the lexer's, with no duplicates.
#[test]
fn editor_keywords_match_lexer() {
    let words = grammar_keywords();
    let set: BTreeSet<&str> = words.iter().map(String::as_str).collect();
    assert_eq!(
        set.len(),
        words.len(),
        "duplicate keyword in editors grammar"
    );
    let lexer: BTreeSet<&str> = KEYWORDS.iter().copied().collect();
    assert_eq!(
        set, lexer,
        "editors/grammar/cadetta.toml drifted from lexer KEYWORDS"
    );
}

/// Absolute path of a file under the repository root.
fn repo(rel: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

/// Read a repository file as text.
fn read(rel: &str) -> String {
    std::fs::read_to_string(repo(rel)).unwrap_or_else(|e| panic!("read {rel}: {e}"))
}

// frob:ticket 01M4VHFFWS5EWZW087G98FFJ8V
/// Committed generated grammars are current (`generate.py --check`) and CI runs the check.
#[test]
fn generated_grammars_are_current() {
    assert!(read(".github/workflows/ci.yml").contains("editors/generate.py --check"));
    let out = std::process::Command::new("uv")
        .args(["run", "editors/generate.py", "--check"])
        .current_dir(repo("."))
        .output();
    match out {
        Ok(o) => assert!(
            o.status.success(),
            "generate.py --check failed: {}",
            String::from_utf8_lossy(&o.stderr)
        ),
        Err(e) => tracing::warn!("skipping generator run, uv unavailable: {e}"),
    }
}

// frob:ticket 01M4VHFFWS5EWZW087G98FFJ8V
/// The VS Code manifest declares the language, grammar, `$cadetta` matcher and task type.
#[test]
fn vscode_manifest_declares_language_matcher_and_task() {
    let pkg: serde_json::Value =
        serde_json::from_str(&read("editors/vscode/package.json")).expect("package.json is JSON");
    let c = &pkg["contributes"];
    assert_eq!(c["languages"][0]["id"], "cadetta");
    assert_eq!(c["languages"][0]["extensions"][0], ".cdt");
    assert_eq!(
        c["grammars"][0]["path"],
        "./syntaxes/cadetta.tmLanguage.json"
    );
    assert_eq!(c["problemMatchers"][0]["name"], "cadetta");
    assert_eq!(c["taskDefinitions"][0]["type"], "cadetta");
    assert!(repo("editors/vscode/language-configuration.json").exists());
    assert!(read("editors/vscode/README.md").contains("vsce package"));
}

// frob:ticket 01M4VHFFWS5EWZW087G98FFJ8V
/// Sublime, Vim, Emacs and JetBrains integrations exist and parse `--format short`.
#[test]
fn other_editors_are_present() {
    for rel in [
        "editors/sublime/Cadetta.sublime-syntax",
        "editors/sublime/Cadetta.sublime-build",
        "editors/sublime/Comments.tmPreferences",
        "editors/vim/ftdetect/cadetta.vim",
        "editors/vim/syntax/cadetta.vim",
        "editors/vim/ftplugin/cadetta.vim",
        "editors/vim/compiler/cadetta.vim",
        "editors/emacs/cadetta-mode.el",
    ] {
        assert!(repo(rel).exists(), "missing {rel}");
    }
    assert!(read("editors/sublime/Cadetta.sublime-build").contains("--format"));
    assert!(read("editors/vim/compiler/cadetta.vim").contains("errorformat"));
    let el = read("editors/emacs/cadetta-mode.el");
    assert!(el.contains("compilation-error-regexp-alist") && el.contains("flymake"));
    assert!(read("editors/jetbrains/README.md").contains("TextMate"));
}

// frob:ticket 01M4VHFFWS5EWZW087G98FFJ8V
/// editors/README.md leads with `cadetta lsp`, names its gaps and keeps the bridges
/// as a fallback, and the root README links it.
#[test]
fn editors_readme_indexes_and_root_links_it() {
    let r = read("editors/README.md");
    for needle in [
        "cadetta lsp",
        "config-cli.md 3.8",
        "nvim-lint",
        "efm-langserver",
        "ALE",
        "flymake",
        "languages.toml",
    ] {
        assert!(r.contains(needle), "editors/README.md lacks {needle}");
    }
    assert!(read("README.md").contains("editors/README.md"));
}
