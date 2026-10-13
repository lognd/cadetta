+++
id = "01M4VHFFWS5EWZW087G98FFJ8V"
title = "editors/: syntax highlighting for VS Code, Sublime, Vim/Neovim, Emacs, JetBrains and diagnostics integration"
type = "story"
category = "in-progress"
priority = "medium"
points = 5
parent = "01M4SPWHHNRSGB9QZ04CMHWRRB"
reporter = "lognd"
created = "2026-10-13T21:07:45Z"
updated = "2026-10-13T21:22:26Z"
persona = "design engineer"
capability = "edit .cdt files with highlighting and inline diagnostics in my editor"
outcome_text = "cadetta feels like a real language from day one"
idempotency_key = "d15:editors"
labels = ["creates:editors/**", "creates:crates/cadetta-cli/tests/editors_keywords.rs"]
scope = ["README.md", ".github/workflows/ci.yml", "editors/**", "crates/cadetta-cli/tests/editors_keywords.rs"]

[[acceptance]]
text = "Given editors/grammar/cadetta.toml, the single list of keyword, contextual-keyword, builtin-type, builtin-function, pseudo-citation and token-pattern classes, when editors/generate.py runs, then it writes every editor's grammar from it, and --check exits non-zero when a committed grammar is stale; CI runs the check"
bound = true

[[acceptance]]
text = "Given crates/cadetta-cli/tests/editors_keywords.rs, when cargo test runs, then the reserved keywords in editors/grammar/cadetta.toml equal cadetta_syntax::lexer::KEYWORDS"
bound = true

[[acceptance]]
text = "Given editors/vscode, when installed per its README (VSIX via npx @vscode/vsce package, or a copy into the extensions folder), then .cdt files get the language id cadetta, highlighting, comment toggling, bracket pairs and a $cadetta problem matcher plus a cadetta check task using --format short"
bound = true

[[acceptance]]
text = "Given editors/sublime, editors/vim (also Neovim), editors/emacs and the JetBrains TextMate-bundle route, when installed per each README, then .cdt files are highlighted and cadetta check --format short diagnostics jump to file:line:col (Sublime build system, Vim compiler/errorformat, Emacs compilation-mode and a flymake backend)"
bound = true

[[acceptance]]
text = "Given editors/README.md, when read, then it indexes the editors, explains that cadetta lsp is deferred (~QB9ANVQ), and recommends diagnostics bridges for now: the VS Code task, nvim-lint or efm-langserver for Neovim and Helix, ALE for Vim, flymake for Emacs, all driving cadetta check --format short; the root README links it"
bound = true
+++

Grammar facts (spec draft 1.5): reserved keywords are language.md 2 (cadetta_syntax::lexer::KEYWORDS); contextual keywords `at` (after a place target), `script` (solver body), `position` (tolerance bracket). Comments `//`, doc comments `///`; attributes `#[...]` and `#![...]`; strings with \" and \\ escapes; citations `@name`, `@a::b`, `@"note"`, optional `:"locator"`; pseudo-citations measured, assumed, estimate; paths with `::`; quantity literals per quantities.md 2 (number, optional fraction `3/4`, unit suffix: `12mm`, `5in`, `0.5mm`, `60m^3/h`); tolerance brackets `[+-0.1mm]`, `[+0.018mm -0]`, `[position 0.2mm]`; ISO 286 classes (`H7`, `p6`, `js6`) after an expression; the built-in types of language.md 4 prelude (Bool Text Count Ratio Point2 Point3 Vec2 Vec3 Datum Table) and kinds of quantities.md; builtin functions of language.md 3.8; cite types (standard datasheet drawing catalog book article web quote document). Diagnostics: `cadetta check --format short` prints `file:line:col: severity[CODE]: message` (lints.md 1.2). Python in the generator follows house rules (module logger, no print except the tool's own output via logging/stdout handler as the repo's Python does, pydantic models for the grammar file, typani Result for fallible loading); run it with uv. Marketplace publishing is out of scope (owner's account); file a ticket for it and for a tree-sitter grammar (Helix, Zed, Neovim treesitter highlighting) instead of building them.
