+++
id = "01M4VQ9HXBNQ6FB1BNE3CZGA04"
title = "editors README test still expects the LSP deferral note (main red)"
type = "bug"
category = "todo"
priority = "high"
points = 1
parent = "01M4SPWHHNRSGB9QZ04CMHWRRB"
reporter = "lognd"
created = "2026-10-13T22:49:22Z"
updated = "2026-10-13T22:50:52Z"
scope = ["crates/cadetta-cli/tests/editors_keywords.rs"]

[[acceptance]]
text = "Given editors/README.md after cadetta lsp landed, when editors_readme_indexes_and_root_links_it runs, then it checks for cadetta lsp, config-cli.md 3.8, the hover ticket ~0CXQ6EZ and the fallback bridges, not the retired deferral ticket, and passes"
bound = false
+++

found while working 606HZK6: editors_readme_indexes_and_root_links_it fails on main
