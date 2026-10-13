+++
id = "01M4SWKA1BZWA1VDN67RQA2FJR"
title = "Short and GitHub renderers"
type = "story"
category = "in-progress"
priority = "medium"
points = 3
parent = "01M4SPWH0ZE4G7KXRDA5M71GPS"
reporter = "lognd"
created = "2026-10-13T05:43:36Z"
updated = "2026-10-13T06:58:08Z"
persona = "design engineer"
capability = "see diagnostics in CI annotations"
idempotency_key = "replan14:lint-short"
labels = ["creates:crates/cadetta-lint/src/render/short.rs", "creates:crates/cadetta-lint/src/render/github.rs"]
scope = ["crates/cadetta-lint/src/render/short.rs", "crates/cadetta-lint/src/render/github.rs", "crates/cadetta-lint/src/render/mod.rs", "crates/cadetta-lint/src/render/text.rs"]

[[links]]
kind = "blocked-by"
target = "01M4SWK9V5CYNNVTAS3PE4EC23"

[[acceptance]]
text = "Given a diagnostic, when rendered short, then it is one line file:line:col: severity[CODE]: message (lints.md 1.2)"
bound = true

[[acceptance]]
text = "Given a diagnostic, when rendered github, then it is a workflow annotation with level mapped from severity (lints.md 1.2)"
bound = false
+++
