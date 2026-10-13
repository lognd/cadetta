+++
id = "01M4SWN8MPH5ARCRJG4QB9ANVQ"
title = "cadetta lsp (deferred)"
type = "story"
category = "todo"
priority = "low"
points = 5
parent = "01M4SPWHHNRSGB9QZ04CMHWRRB"
reporter = "lognd"
created = "2026-10-13T05:44:40Z"
updated = "2026-10-13T05:44:40Z"
persona = "design engineer"
capability = "see problems in any editor"
idempotency_key = "replan14:cli-lsp"
labels = ["deferred", "creates:crates/cadetta-cli/src/cmd/lsp.rs"]
scope = ["crates/cadetta-cli/src/cmd/lsp.rs"]

[[acceptance]]
text = "Given the deferred lsp command, when implemented after 0.1.0, then it serves cadetta.diag/1 diagnostics over the language server protocol (config-cli.md 3)"
bound = false
+++
