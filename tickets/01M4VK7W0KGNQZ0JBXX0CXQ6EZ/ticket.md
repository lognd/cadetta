+++
id = "01M4VK7W0KGNQZ0JBXX0CXQ6EZ"
title = "cadetta lsp: hover as why, code lens as impact, go to definition"
type = "story"
category = "in-progress"
priority = "low"
points = 5
parent = "01M4SPWHHNRSGB9QZ04CMHWRRB"
reporter = "lognd"
created = "2026-10-13T21:38:33Z"
updated = "2026-10-17T00:08:27Z"
persona = "design engineer"
capability = "see a value's derivation and impact from the editor"
outcome_text = "why and impact are one hover away"
idempotency_key = "d15:lsp-queries"
scope = ["crates/cadetta-cli/src/cmd/lsp.rs", "crates/cadetta-cli/src/cmd/lsp_query.rs", "crates/cadetta-cli/src/cmd/mod.rs", "crates/cadetta-cli/tests/lsp.rs", "docs/spec/config-cli.md"]

[[acceptance]]
text = "Given the 0.1.0 lsp, when extended after 0.1.0, then hover shows why, a code lens shows impact and go to definition follows canonical paths, specified in config-cli.md 3.8 first"
bound = true
+++
