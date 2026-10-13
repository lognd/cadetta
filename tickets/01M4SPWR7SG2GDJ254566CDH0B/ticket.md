+++
id = "01M4SPWR7SG2GDJ254566CDH0B"
title = "Lint family Check rules C001-C004"
type = "story"
category = "in-progress"
priority = "medium"
points = 3
parent = "01M4SPWH0ZE4G7KXRDA5M71GPS"
reporter = "lognd"
created = "2026-10-13T04:03:54Z"
updated = "2026-10-13T09:45:43Z"
persona = "design engineer"
capability = "get lints for the C family of the catalog"
outcome_text = "I see actionable diagnostics with stable codes"
labels = ["creates:crates/cadetta-lint/src/rules/c.rs", "creates:tests/conformance/lints/C*/**"]
scope = ["crates/cadetta-lint/src/rules/c.rs", "tests/conformance/lints/C*/**"]

[[acceptance]]
text = "Given a check violated across the tolerance, when linted, then C001 error"
bound = false

[[acceptance]]
text = "Given an undecided check, when linted, then C002 warning"
bound = false

[[acceptance]]
text = "Given `a == b` on toleranced operands, when linted, then C003 warning suggests `.nominal` accessors (quantities.md 5.8)"
bound = false

[[acceptance]]
text = "Given a check reading an arithmetic parameter with toleranced inputs and no tolerance of its own, when linted, then C004 info suggests `let` (quantities.md 5.3)"
bound = false

[[acceptance]]
text = "Given a violation under an alternate, when linted, then C001 names `for alt NAME` (language.md 3.2)"
bound = false

[[acceptance]]
text = "Given the rule, when shipped, then a must-fire and a must-not-fire fixture exist under tests/conformance/lints/<code>/"
bound = false
+++
