+++
id = "01M4SPWQZF9T7HA75ESE6D2C96"
title = "Lint family Magic-number rules M001-M003"
type = "story"
category = "in-progress"
priority = "medium"
points = 5
parent = "01M4SPWH0ZE4G7KXRDA5M71GPS"
reporter = "lognd"
created = "2026-10-13T04:03:53Z"
updated = "2026-10-13T13:23:17Z"
persona = "design engineer"
capability = "get lints for the M family of the catalog"
outcome_text = "I see actionable diagnostics with stable codes"
labels = ["creates:crates/cadetta-lint/src/rules/m.rs", "creates:tests/conformance/lints/M*/**"]
scope = ["crates/cadetta-lint/src/rules/m.rs", "tests/conformance/lints/M*/**", "crates/cadetta-lint/src/rules/mod.rs", "docs/spec/lints.md"]

[[acceptance]]
text = "Given `12.012mm` in a model with no tolerance, fit, table lookup or allow, when linted, then M001 is reported; a citation or doc comment does not explain it (lints.md M)"
bound = true

[[acceptance]]
text = "Given the same literal in an ots with a citation, when linted, then M001 does not fire (lints.md M)"
bound = true

[[acceptance]]
text = "Given a bare factor other than small integers, simple fractions and powers of ten, when linted, then M002 info (lints.md M)"
bound = true

[[acceptance]]
text = "Given a parameter used as a fit side with no tolerance, when linted, then M003 is reported (lints.md M)"
bound = false
+++
