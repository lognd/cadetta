+++
id = "01M4SPWQPYYT8Y5N3P5ZSG20Z9"
title = "Lint family Quantity rules Q001-Q007"
type = "story"
category = "todo"
priority = "medium"
points = 3
parent = "01M4SPWH0ZE4G7KXRDA5M71GPS"
reporter = "lognd"
created = "2026-10-13T04:03:53Z"
updated = "2026-10-13T04:03:53Z"
persona = "design engineer"
capability = "get lints for the Q family of the catalog"
outcome_text = "I see actionable diagnostics with stable codes"
labels = ["creates:crates/cadetta-lint/src/rules/q.rs", "creates:tests/lints/Q*/**"]
scope = ["crates/cadetta-lint/src/rules/q.rs", "tests/lints/Q*/**"]

[[acceptance]]
text = "Given `x: Length = 5`, when linted, then Q001 error is reported"
bound = false

[[acceptance]]
text = "Given `5 mm`, when linted, then Q007 is reported and cannot be waived"
bound = false

[[acceptance]]
text = "Given the rule, when shipped, then positive and negative fixtures exist under tests/lints/<code>/"
bound = false
+++
