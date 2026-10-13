+++
id = "01M4SPWQWV4J8DF3B1RF4PFYCT"
title = "Lint family Duplication rules D001-D003"
type = "story"
category = "todo"
priority = "medium"
points = 5
parent = "01M4SPWH0ZE4G7KXRDA5M71GPS"
reporter = "lognd"
created = "2026-10-13T04:03:53Z"
updated = "2026-10-13T04:03:53Z"
persona = "design engineer"
capability = "get lints for the D family of the catalog"
outcome_text = "I see actionable diagnostics with stable codes"
labels = ["creates:crates/cadetta-lint/src/rules/d.rs", "creates:tests/lints/D*/**"]
scope = ["crates/cadetta-lint/src/rules/d.rs", "tests/lints/D*/**"]

[[acceptance]]
text = "Given `1mm` repeated across components and not in common_values, when linted, then D001 warning"
bound = false

[[acceptance]]
text = "Given `1mm` in common_values, when linted, then D001 is info"
bound = false

[[acceptance]]
text = "Given a literal exactly twice an existing value, when linted, then D002 is reported"
bound = false

[[acceptance]]
text = "Given the rule, when shipped, then positive and negative fixtures exist under tests/lints/<code>/"
bound = false
+++
