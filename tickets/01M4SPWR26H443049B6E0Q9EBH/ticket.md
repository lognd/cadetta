+++
id = "01M4SPWR26H443049B6E0Q9EBH"
title = "Lint family Fit rules F001-F004"
type = "story"
category = "todo"
priority = "medium"
points = 5
parent = "01M4SPWH0ZE4G7KXRDA5M71GPS"
reporter = "lognd"
created = "2026-10-13T04:03:54Z"
updated = "2026-10-13T05:01:24Z"
persona = "design engineer"
capability = "get lints for the F family of the catalog"
outcome_text = "I see actionable diagnostics with stable codes"
labels = ["creates:crates/cadetta-lint/src/rules/f.rs", "creates:tests/lints/F*/**"]
scope = ["crates/cadetta-lint/src/rules/f.rs", "tests/lints/F*/**"]

[[acceptance]]
text = "Given class Z99, when linted, then F001 is reported and cannot be waived"
bound = false

[[acceptance]]
text = "Given a fitted parameter whose tolerance disagrees with the hole side, when linted, then F003 is reported"
bound = false

[[acceptance]]
text = "Given the rule, when shipped, then positive and negative fixtures exist under tests/lints/<code>/"
bound = false

[[acceptance]]
text = "Given a fit whose clearance lies entirely outside its required range, when linted, then F004 error; a partial overlap yields C002 instead (lints.md F)"
bound = false

[[acceptance]]
text = "Given a nominal outside the table size range, when linted, then F002 is reported and cannot be waived (quantities.md 5.1)"
bound = false
+++
