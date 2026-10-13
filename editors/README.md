# Editor support

Syntax highlighting and `cadetta check` integration for `.cdt` files.
Every grammar is generated from one source, `grammar/cadetta.toml`, by
`generate.py`; do not edit generated files. After changing the grammar run

    uv run editors/generate.py          # rewrite outputs
    uv run editors/generate.py --check  # CI: fail if outputs are stale

A Rust test (`crates/cadetta-cli/tests/editors_keywords.rs`) keeps the
keyword list equal to the lexer's.

| Editor | Directory |
|---|---|
| VS Code | [vscode](vscode/README.md) |
| Sublime Text | [sublime](sublime/README.md) |
| Vim / Neovim | [vim](vim/README.md) |
| Emacs | [emacs](emacs/README.md) |
| JetBrains | [jetbrains](jetbrains/README.md) |

## Language server status

`cadetta lsp` is deferred (ticket ~QB9ANVQ). Until then, get live diagnostics
by running `cadetta check --format short` (lines of the form
`file:line:col: severity[CODE]: message`) through a bridge.

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

Helix has no TextMate support; highlighting there needs a tree-sitter grammar
(not yet written).

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
