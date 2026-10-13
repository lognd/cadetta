+++
id = "01M4VJX268P44QDV84VHGBP4YN"
title = "Spec: cadetta lsp (diagnostics and formatting) in 0.1.0"
type = "docs"
category = "in-progress"
priority = "medium"
points = 2
parent = "01M4VB1MJC8NTMD6AAZR5Y5J08"
reporter = "lognd"
created = "2026-10-13T21:32:38Z"
updated = "2026-10-13T21:33:32Z"
idempotency_key = "d15:lsp-spec"
scope = ["docs/spec/config-cli.md", "docs/spec/lints.md", "docs/spec/README.md", "docs/decisions/2026-10-13-draft-1-5-decisions.md"]

[[acceptance]]
text = "Given config-cli.md and lints.md, when read, then cadetta lsp is a 0.1.0 command serving diagnostics (live syntax errors on change, full check on save) and formatting over LSP, with the cadetta.diag/1 mapping, position encoding and lifecycle specified, and hover/code lens deferred"
bound = true

[[acceptance]]
text = "Given the draft 1.5 ADR, when read, then it records bringing the LSP into 0.1.0 and why"
bound = false
+++
