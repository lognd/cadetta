+++
id = "01M4SPWR7SG2GDJ254566CDH0B"
title = "Lint family Check rules C001-C002"
type = "story"
category = "todo"
priority = "medium"
points = 2
parent = "01M4SPWH0ZE4G7KXRDA5M71GPS"
reporter = "lognd"
created = "2026-10-13T04:03:54Z"
updated = "2026-10-13T04:03:54Z"
persona = "design engineer"
capability = "get lints for the C family of the catalog"
outcome_text = "I see actionable diagnostics with stable codes"
labels = ["creates:crates/cadetta-lint/src/rules/c.rs", "creates:tests/lints/C*/**"]
scope = ["crates/cadetta-lint/src/rules/c.rs", "tests/lints/C*/**"]

[[acceptance]]
text = "Given a check violated across the tolerance, when linted, then C001 error"
bound = false

[[acceptance]]
text = "Given an undecided check, when linted, then C002 warning"
bound = false

[[acceptance]]
text = "Given the rule, when shipped, then positive and negative fixtures exist under tests/lints/<code>/"
bound = false
+++
