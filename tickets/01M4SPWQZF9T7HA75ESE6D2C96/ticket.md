+++
id = "01M4SPWQZF9T7HA75ESE6D2C96"
title = "Lint family Magic-number rules M001-M003"
type = "story"
category = "todo"
priority = "medium"
points = 5
parent = "01M4SPWH0ZE4G7KXRDA5M71GPS"
reporter = "lognd"
created = "2026-10-13T04:03:53Z"
updated = "2026-10-13T04:03:53Z"
persona = "design engineer"
capability = "get lints for the M family of the catalog"
outcome_text = "I see actionable diagnostics with stable codes"
labels = ["creates:crates/cadetta-lint/src/rules/m.rs", "creates:tests/lints/M*/**"]
scope = ["crates/cadetta-lint/src/rules/m.rs", "tests/lints/M*/**"]

[[acceptance]]
text = "Given `12.012mm` with no tolerance, fit, source or doc, when linted, then M001 is reported"
bound = false

[[acceptance]]
text = "Given a parameter used by a fit with no tolerance, when linted, then M003 is reported"
bound = false

[[acceptance]]
text = "Given the rule, when shipped, then positive and negative fixtures exist under tests/lints/<code>/"
bound = false
+++
