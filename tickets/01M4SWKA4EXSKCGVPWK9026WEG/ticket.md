+++
id = "01M4SWKA4EXSKCGVPWK9026WEG"
title = "SARIF 2.1.0 renderer"
type = "story"
category = "todo"
priority = "medium"
points = 5
parent = "01M4SPWH0ZE4G7KXRDA5M71GPS"
reporter = "lognd"
created = "2026-10-13T05:43:36Z"
updated = "2026-10-13T05:43:36Z"
persona = "design engineer"
capability = "upload results to code scanning"
idempotency_key = "replan14:lint-sarif"
labels = ["creates:crates/cadetta-lint/src/render/sarif.rs"]
scope = ["crates/cadetta-lint/src/render/sarif.rs"]

[[acceptance]]
text = "Given diagnostics, when rendered sarif, then the output validates against SARIF 2.1.0 with rules from the catalog (lints.md 1.2)"
bound = false

[[acceptance]]
text = "Given native or lock locations, when rendered, then they map to artifact locations without losing the path (lints.md 1.1, 1.2)"
bound = false
+++
