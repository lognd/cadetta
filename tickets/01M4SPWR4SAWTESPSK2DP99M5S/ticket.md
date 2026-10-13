+++
id = "01M4SPWR4SAWTESPSK2DP99M5S"
title = "Lint family Source rules S001-S002"
type = "story"
category = "todo"
priority = "medium"
points = 2
parent = "01M4SPWH0ZE4G7KXRDA5M71GPS"
reporter = "lognd"
created = "2026-10-13T04:03:54Z"
updated = "2026-10-13T04:03:54Z"
persona = "design engineer"
capability = "get lints for the S family of the catalog"
outcome_text = "I see actionable diagnostics with stable codes"
labels = ["creates:crates/cadetta-lint/src/rules/s.rs", "creates:tests/lints/S*/**"]
scope = ["crates/cadetta-lint/src/rules/s.rs", "tests/lints/S*/**"]

[[acceptance]]
text = "Given an ots value with no source on it or its component, when linted, then S001 warning"
bound = false

[[acceptance]]
text = "Given a local-file source that does not exist, when linted, then S002 info"
bound = false

[[acceptance]]
text = "Given the rule, when shipped, then positive and negative fixtures exist under tests/lints/<code>/"
bound = false
+++
