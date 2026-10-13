+++
id = "01M4SPWRAWXJTWT4G08578RNQM"
title = "Lint family Drift rules R001-R004 (needs observed IR)"
type = "story"
category = "todo"
priority = "medium"
points = 5
parent = "01M4SPWH0ZE4G7KXRDA5M71GPS"
reporter = "lognd"
created = "2026-10-13T04:03:54Z"
updated = "2026-10-13T04:03:54Z"
persona = "design engineer"
capability = "get lints for the R family of the catalog"
outcome_text = "I see actionable diagnostics with stable codes"
labels = ["creates:crates/cadetta-lint/src/rules/r.rs", "creates:tests/lints/R*/**"]
scope = ["crates/cadetta-lint/src/rules/r.rs", "tests/lints/R*/**"]

[[acceptance]]
text = "Given an observed value differing from design at a bound path, when linted, then R001 names which side moved since last sync"
bound = false

[[acceptance]]
text = "Given a native parameter with no binding, when linted, then R002 warning"
bound = false

[[acceptance]]
text = "Given a bound path absent natively, when linted, then R003 error"
bound = false

[[acceptance]]
text = "Given the rule, when shipped, then positive and negative fixtures exist under tests/lints/<code>/"
bound = false
+++
