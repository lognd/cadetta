+++
id = "01M4SWK9Y67QRHQ6K0FMZMDBM4"
title = "Text renderer, rustc-style"
type = "story"
category = "todo"
priority = "medium"
points = 5
parent = "01M4SPWH0ZE4G7KXRDA5M71GPS"
reporter = "lognd"
created = "2026-10-13T05:43:36Z"
updated = "2026-10-13T05:43:36Z"
persona = "design engineer"
capability = "read diagnostics like rustc output"
idempotency_key = "replan14:lint-text"
labels = ["creates:crates/cadetta-lint/src/render/text.rs"]
scope = ["crates/cadetta-lint/src/render/text.rs"]

[[acceptance]]
text = "Given a diagnostic with source span, notes, fixes and path, when rendered as text, then it prints severity[CODE], --> file:line:col, an excerpt with carets and = note:, = help:, = path: lines (lints.md 1.2)"
bound = false

[[acceptance]]
text = "Given no terminal, when rendering, then text is still the default (lints.md 1.2; config-cli.md 5)"
bound = false
+++
