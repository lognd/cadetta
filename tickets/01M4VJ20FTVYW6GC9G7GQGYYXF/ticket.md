+++
id = "01M4VJ20FTVYW6GC9G7GQGYYXF"
title = "editors/: tree-sitter grammar for Helix, Zed and Neovim treesitter"
type = "story"
category = "in-progress"
priority = "medium"
points = 5
parent = "01M4SPWHHNRSGB9QZ04CMHWRRB"
reporter = "lognd"
created = "2026-10-13T21:17:52Z"
updated = "2026-10-16T23:44:04Z"
persona = "a designer using Helix, Zed or Neovim treesitter"
capability = "highlight .cdt files with a tree-sitter grammar"
outcome_text = "editors without TextMate support get accurate highlighting"
scope = ["editors/tree-sitter/**", "editors/generate.py", "editors/grammar/cadetta.toml", "editors/README.md"]

[[acceptance]]
text = "Given editors/tree-sitter, when tree-sitter generate and tree-sitter test run, then the grammar parses every tests/golden/*/src/*.cdt without ERROR nodes and ships highlights.scm queries"
bound = false

[[acceptance]]
text = "Given editors/README.md, when read, then it documents Helix, Zed and Neovim treesitter installation of the grammar"
bound = false
+++

found while working ~98FFJ8V; TextMate grammars there cannot serve Helix or Zed.
