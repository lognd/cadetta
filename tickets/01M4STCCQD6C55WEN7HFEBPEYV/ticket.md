+++
id = "01M4STCCQD6C55WEN7HFEBPEYV"
title = "Lints L001-L002, T001 and P001"
type = "story"
category = "in-progress"
priority = "medium"
points = 3
parent = "01M4SPWH0ZE4G7KXRDA5M71GPS"
reporter = "lognd"
created = "2026-10-13T05:04:52Z"
updated = "2026-10-13T13:32:20Z"
persona = "design engineer"
capability = "be warned about lifecycle, missing table rows and process roles"
idempotency_key = "replan13:Lints L001-L002, T001 and P001"
labels = ["creates:crates/cadetta-lint/src/rules/lifecycle.rs"]
scope = ["crates/cadetta-lint/src/rules/lifecycle.rs", "crates/cadetta-lint/src/rules/mod.rs", "docs/spec/lints.md"]

[[acceptance]]
text = "Given L001 and L002, when linted, then they fire per the catalog, L001 an error once the date passed (lints.md L)"
bound = true

[[acceptance]]
text = "Given T001, when an ots literal matches no row, then it fires (lints.md T)"
bound = false

[[acceptance]]
text = "Given P001, when a feature-like parameter has no role under a process, then it fires (lints.md P)"
bound = false
+++
