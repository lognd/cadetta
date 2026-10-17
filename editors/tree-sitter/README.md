# Cadetta tree-sitter grammar

For editors that highlight with tree-sitter rather than TextMate: Helix, Zed
and Neovim (nvim-treesitter). The language server (`cadetta lsp`, see
[../README.md](../README.md)) is separate.

The grammar is lexical: a `.cdt` file is a flat stream of tokens (comments,
strings, attributes, citations, quantities, numbers, keywords, identifiers,
operators, punctuation) and every character lexes, so no input yields `ERROR`
nodes. Highlighting does the semantic work in `queries/highlights.scm`
(contextual words such as `at` after `place`, cite types, ISO 286 classes).
`grammar.js` and `queries/highlights.scm` are generated from
`../grammar/cadetta.toml` by `../generate.py`; do not edit them. Hand-written:
`tree-sitter.json`, `package.json`, `test/corpus/`, `check_golden.py`.

Known limits: `H7/p6` style classes highlight only the first part; `mass`-like
builtin names highlight wherever they appear as identifiers.

## Build and test

Needs the tree-sitter CLI (0.25 or newer), Node.js and a C compiler:

    cd editors/tree-sitter
    tree-sitter generate                        # writes src/ (git-ignored)
    tree-sitter test                            # test/corpus
    uv run check_golden.py                      # tests/golden/*/src/*.cdt: no ERROR nodes

## Helix

Helix builds grammars from a source with `src/parser.c`, so run
`tree-sitter generate` first, then in `~/.config/helix/languages.toml` (merge
with the language entry from [../README.md](../README.md)):

    [[grammar]]
    name = "cadetta"
    source = { path = "/path/to/cadetta/editors/tree-sitter" }

Run `hx --grammar build`, then copy the queries:

    mkdir -p ~/.config/helix/runtime/queries/cadetta
    cp queries/highlights.scm ~/.config/helix/runtime/queries/cadetta/

## Zed

Zed grammars come from an extension. Create a dev extension directory with:

    extension.toml
        id = "cadetta"
        name = "Cadetta"
        version = "0.1.0"
        schema_version = 1
        authors = ["lognd"]
        description = "Cadetta language support"
        repository = "https://github.com/lognd/cadetta"

        [grammars.cadetta]
        repository = "https://github.com/lognd/cadetta"
        rev = "<commit that contains editors/tree-sitter>"
        path = "editors/tree-sitter"

    languages/cadetta/config.toml
        name = "Cadetta"
        grammar = "cadetta"
        path_suffixes = ["cdt"]
        line_comments = ["// ", "/// "]

    languages/cadetta/highlights.scm   (copy of queries/highlights.scm)

then run "zed: install dev extension" and pick the directory. Zed falls back
from dotted captures (`keyword.control`) to their prefix.

## Neovim (nvim-treesitter)

    vim.filetype.add({ extension = { cdt = "cadetta" } })

    vim.api.nvim_create_autocmd("User", {
      pattern = "TSUpdate",
      callback = function()
        require("nvim-treesitter.parsers").cadetta = {
          install_info = {
            url = "https://github.com/lognd/cadetta",
            location = "editors/tree-sitter",
            generate = true,
            queries = "queries",
          },
        }
      end,
    })

Then `:TSInstall cadetta` and `vim.treesitter.start()` (or enable highlight in
your nvim-treesitter setup). For the older `master` branch use
`parser_config = require("nvim-treesitter.parsers").get_parser_configs()` with
`install_info = { url = ..., location = "editors/tree-sitter", files = { "src/parser.c" }, generate_requires_npm = false, requires_generate_from_grammar = true }`
and copy `queries/highlights.scm` to `~/.config/nvim/queries/cadetta/`.
