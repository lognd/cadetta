+++
id = "01M4SPWQPYYT8Y5N3P5ZSG20Z9"
title = "Lint family Quantity rules Q001-Q011"
type = "story"
category = "in-progress"
priority = "medium"
points = 5
parent = "01M4SPWH0ZE4G7KXRDA5M71GPS"
reporter = "lognd"
created = "2026-10-13T04:03:53Z"
updated = "2026-10-13T12:53:53Z"
persona = "design engineer"
capability = "get lints for the Q family of the catalog"
outcome_text = "I see actionable diagnostics with stable codes"
labels = ["creates:crates/cadetta-lint/src/rules/q.rs", "creates:tests/conformance/lints/Q*/**"]
scope = ["crates/cadetta-lint/src/rules/q.rs", "tests/conformance/lints/Q*/**"]

[[acceptance]]
text = "Given `x: Length = 5`, when linted, then Q001 error is reported (lints.md Q)"
bound = true

[[acceptance]]
text = "Given `5 mm`, when linted, then Q007 is reported with a fix-it and cannot be waived (lints.md Q)"
bound = false

[[acceptance]]
text = "Given codes Q001-Q003, Q007, Q009, Q010 and Q011, when severity is overridden, then the override is ignored because they are fixed (lints.md 1, Q)"
bound = false

[[acceptance]]
text = "Given Q004, Q006 and Q008, when emitted, then default severities are warning, info and warning and Q006 is reported once at the root inexact operation with the dependent count (lints.md Q)"
bound = false

[[acceptance]]
text = "Given Q005, when queried, then it is listed as retired and never emitted (lints.md Q)"
bound = false
+++
