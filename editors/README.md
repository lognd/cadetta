# Editor support

Syntax highlighting and language-server (`cadetta lsp`) integration for `.cdt` files.
Every grammar is generated from one source, `grammar/cadetta.toml`, by
`generate.py`; do not edit generated files. After changing the grammar run

    uv run editors/generate.py          # rewrite outputs
    uv run editors/generate.py --check  # CI: fail if outputs are stale

The tree-sitter `grammar.js` and `queries/highlights.scm` are generated too;
`tree-sitter generate`, `tree-sitter test` and `check_golden.py` (see
[tree-sitter](tree-sitter/README.md)) verify them. A Rust test (`crates/cadetta-cli/tests/editors_keywords.rs`) keeps the
keyword list equal to the lexer's.

| Editor | Directory |
|---|---|
| VS Code | [vscode](vscode/README.md) |
| Sublime Text | [sublime](sublime/README.md) |
| Vim / Neovim | [vim](vim/README.md) |
| Emacs | [emacs](emacs/README.md) |
| JetBrains | [jetbrains](jetbrains/README.md) |
| Helix, Zed, Neovim treesitter | [tree-sitter](tree-sitter/README.md) |

## Language server (primary)

`cadetta lsp` is a Language Server Protocol 3.17 server on stdio (spec
config-cli.md 3.8). It publishes the same diagnostics as `cadetta check` and
formats a buffer like `cadetta fmt`. It also provides hover (the `why` chain), a code lens
(`impact: N dependents`, command `cadetta.impact`) and go to definition;
completion is not yet provided. Every editor
below starts `cadetta lsp` for `*.cdt` files with root markers
`cadetta.toml` and `.git`; the command is always `cadetta` with the single
argument `lsp` (it takes no other arguments). Highlighting still comes from
the grammars in this directory.

### Neovim (0.11+): vim.lsp.config

    vim.lsp.config("cadetta", {
      cmd = { "cadetta", "lsp" },
      filetypes = { "cadetta" },
      root_markers = { "cadetta.toml", ".git" },
    })
    vim.lsp.enable("cadetta")

Neovim needs the `cadetta` filetype; add `editors/vim` to the runtimepath
(see [vim](vim/README.md)) or `vim.filetype.add({ extension = { cdt = "cadetta" } })`.

### Neovim (older): nvim-lspconfig custom config

    local lspconfig = require("lspconfig")
    local configs = require("lspconfig.configs")
    if not configs.cadetta then
      configs.cadetta = { default_config = {
        cmd = { "cadetta", "lsp" },
        filetypes = { "cadetta" },
        root_dir = lspconfig.util.root_pattern("cadetta.toml", ".git"),
      } }
    end
    lspconfig.cadetta.setup({})

### Helix: languages.toml

    [language-server.cadetta]
    command = "cadetta"
    args = ["lsp"]

    [[language]]
    name = "cadetta"
    scope = "source.cadetta"
    file-types = ["cdt"]
    roots = ["cadetta.toml"]
    comment-token = "//"
    language-servers = ["cadetta"]
    auto-format = true

Helix has no TextMate support; its highlighting comes from the tree-sitter
grammar in [tree-sitter](tree-sitter/README.md).

### Emacs: eglot

    (add-to-list 'eglot-server-programs '(cadetta-mode . ("cadetta" "lsp")))
    (add-hook 'cadetta-mode-hook #'eglot-ensure)

with `cadetta-mode` loaded from [emacs](emacs/README.md). Format with
`M-x eglot-format-buffer`.

### Sublime Text: LSP package

Install the `LSP` package, then in Preferences > Package Settings > LSP >
Settings add:

    {
      "clients": {
        "cadetta": {
          "enabled": true,
          "command": ["cadetta", "lsp"],
          "selector": "source.cadetta"
        }
      }
    }

`source.cadetta` is the scope of the syntax in [sublime](sublime/README.md).

### Vim: vim-lsp

    if executable('cadetta')
      autocmd User lsp_setup call lsp#register_server({
      \ 'name': 'cadetta',
      \ 'cmd': {server_info -> ['cadetta', 'lsp']},
      \ 'allowlist': ['cadetta'],
      \ })
    endif

### Vim: ALE LSP linter

    call ale#linter#Define('cadetta', {
    \ 'name': 'cadetta-lsp',
    \ 'lsp': 'stdio',
    \ 'executable': 'cadetta',
    \ 'command': '%e lsp',
    \ 'project_root': {buffer -> ale#path#FindNearestFile(buffer, 'cadetta.toml')
    \                           ->fnamemodify(':h')},
    \ })
    let g:ale_linters = { 'cadetta': ['cadetta-lsp'] }

### VS Code and JetBrains

VS Code ships a language client ([vscode](vscode/README.md)); JetBrains uses
the LSP4IJ plugin with a custom server ([jetbrains](jetbrains/README.md)).

## Fallback: `cadetta check --format short` bridges

Use these only where no LSP client is available. They run `cadetta check
--format short` (lines of the form `file:line:col: severity[CODE]: message`)
through a bridge and re-check the whole package on each run.

### Neovim: nvim-lint

    local lint = require("lint")
    lint.linters.cadetta = {
      cmd = "cadetta", args = { "check", "--format", "short" }, stdin = false,
      stream = "stdout", ignore_exitcode = true,
      parser = require("lint.parser").from_errorformat(
        "%f:%l:%c: %t%*[a-z][%*[^]]]: %m", { source = "cadetta" }),
    }
    lint.linters_by_ft = { cadetta = { "cadetta" } }
    vim.api.nvim_create_autocmd({ "BufWritePost" }, {
      callback = function() lint.try_lint() end })

### Neovim: efm-langserver

    require("lspconfig").efm.setup({
      filetypes = { "cadetta" },
      settings = { languages = { cadetta = { {
        lintCommand = "cadetta check --format short",
        lintFormats = { "%f:%l:%c: %trror[%*[^]]]: %m",
                        "%f:%l:%c: %tarning[%*[^]]]: %m" },
        lintIgnoreExitCode = true,
      } } } },
    })

### Helix: languages.toml (with efm-langserver)

    [language-server.efm]
    command = "efm-langserver"

    [[language]]
    name = "cadetta"
    scope = "source.cadetta"
    file-types = ["cdt"]
    comment-token = "//"
    language-servers = ["efm"]

with `~/.config/efm-langserver/config.yaml`:

    languages:
      cadetta:
        lint-command: "cadetta check --format short"
        lint-ignore-exit-code: true
        lint-formats:
          - "%f:%l:%c: %trror[%*[^]]]: %m"
          - "%f:%l:%c: %tarning[%*[^]]]: %m"

Helix has no TextMate support; its highlighting comes from the tree-sitter
grammar in [tree-sitter](tree-sitter/README.md).

### Vim: ALE

    function! CadettaHandle(buffer, lines) abort
      let l:pat = '\v^(.{-}):(\d+):(\d+): (\w+)\[(\w+)\]: (.*)$'
      let l:out = []
      for l:m in ale#util#GetMatches(a:lines, l:pat)
        call add(l:out, {'lnum': l:m[2], 'col': l:m[3], 'code': l:m[5],
        \ 'text': l:m[6], 'type': l:m[4] is# 'error' ? 'E' : 'W'})
      endfor
      return l:out
    endfunction
    call ale#linter#Define('cadetta', {
    \ 'name': 'cadetta', 'executable': 'cadetta',
    \ 'command': 'cadetta check --format short',
    \ 'lint_file': 1, 'output_stream': 'stdout',
    \ 'callback': 'CadettaHandle',
    \ })
    let g:ale_linters = { 'cadetta': ['cadetta'] }

### Emacs: flymake

`editors/emacs/cadetta-mode.el` already registers `cadetta-flymake`; enable
with `M-x flymake-mode`.

## Known highlighting limits

ISO tolerance classes after a quantity are matched heuristically; names such
as `mass` highlight as builtin functions even when used as field names.
