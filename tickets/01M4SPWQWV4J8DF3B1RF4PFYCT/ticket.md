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
updated = "2026-10-13T05:46:22Z"
persona = "design engineer"
capability = "get lints for the D family of the catalog"
outcome_text = "I see actionable diagnostics with stable codes"
labels = ["creates:crates/cadetta-lint/src/rules/d.rs", "creates:tests/lints/D*/**"]
scope = ["crates/cadetta-lint/src/rules/d.rs", "tests/lints/D*/**"]

[[acceptance]]
text = "Given the literal 12.012mm repeated in a model with at least d001_min_digits significant digits, when linted, then one D001 is reported listing every location (lints.md D)"
bound = false

[[acceptance]]
text = "Given `100mm` repeated (1 significant digit), when linted, then D001 is info; given a value in common_values then no D001 unless either side is imported (lints.md D)"
bound = false

[[acceptance]]
text = "Given two ots parameters with equal values, when linted, then D001 does not fire (lints.md D)"
bound = false

[[acceptance]]
text = "Given `12.009mm` equal to `seat.shaft.mid`, when linted, then D002 is reported (lints.md D)"
bound = false

[[acceptance]]
text = "Given a literal equal to a table cell reachable from the component or a solver output, when linted, then D002 is reported (lints.md D)"
bound = false

[[acceptance]]
text = "Given a cycle between members, when linted, then D003 is an error and cannot be waived; a component-level but not member-level loop is clean (language.md 4)"
bound = false

[[acceptance]]
text = "Given each code, when shipped, then a must-fire and a must-not-fire fixture exist under tests/conformance/lints/<code>/"
bound = false
+++
