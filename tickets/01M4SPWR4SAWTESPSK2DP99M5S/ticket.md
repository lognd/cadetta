+++
id = "01M4SPWR4SAWTESPSK2DP99M5S"
title = "Lint family Source rules S001-S002"
type = "story"
category = "in-progress"
priority = "medium"
points = 2
parent = "01M4SPWH0ZE4G7KXRDA5M71GPS"
reporter = "lognd"
created = "2026-10-13T04:03:54Z"
updated = "2026-10-13T13:24:31Z"
persona = "design engineer"
capability = "get lints for the S family of the catalog"
outcome_text = "I see actionable diagnostics with stable codes"
labels = ["creates:crates/cadetta-lint/src/rules/s.rs", "creates:tests/conformance/lints/S*/**"]
scope = ["crates/cadetta-lint/src/rules/s.rs", "tests/conformance/lints/S*/**", "crates/cadetta-lint/src/rules/mod.rs"]

[[acceptance]]
text = "Given an ots value with no source on it or its component, when linted, then S001 warning"
bound = true

[[acceptance]]
text = "Given a local-file source that does not exist, when linted, then S002 info"
bound = true
+++
