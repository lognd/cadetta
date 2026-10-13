+++
id = "01M4VK7WCP2FMEAHHJGTJX4YYF"
title = "editors/: use cadetta lsp in every editor config"
type = "story"
category = "in-progress"
priority = "medium"
points = 3
parent = "01M4SPWHHNRSGB9QZ04CMHWRRB"
reporter = "lognd"
created = "2026-10-13T21:38:33Z"
updated = "2026-10-13T22:35:55Z"
persona = "design engineer"
capability = "get diagnostics and formatting through one standard LSP setup"
outcome_text = "no per-editor bridge configuration is needed"
idempotency_key = "d15:editors-lsp"
scope = ["editors/**", ".github/workflows/ci.yml"]

[[links]]
kind = "blocked-by"
target = "01M4SWN8MPH5ARCRJG4QB9ANVQ"

[[acceptance]]
text = "Given editors/vscode, when installed, then a minimal language client (vscode-languageclient) starts cadetta lsp for .cdt files, with a setting for the cadetta path, and the task/problem matcher stays as a fallback"
bound = true

[[acceptance]]
text = "Given editors/README.md and each editor README, when read, then Neovim (vim.lsp.config), Helix (languages.toml), Emacs (eglot), Sublime (LSP package), Vim (vim-lsp or ALE LSP) and JetBrains (LSP4IJ) start cadetta lsp, and the bridge recipes are kept only as a fallback (config-cli.md 3.8)"
bound = true
+++
