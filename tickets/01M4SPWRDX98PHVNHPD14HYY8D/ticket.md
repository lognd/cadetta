+++
id = "01M4SPWRDX98PHVNHPD14HYY8D"
title = "Lint family Waiver rules W001-W004"
type = "story"
category = "todo"
priority = "medium"
points = 3
parent = "01M4SPWH0ZE4G7KXRDA5M71GPS"
reporter = "lognd"
created = "2026-10-13T04:03:54Z"
updated = "2026-10-13T05:46:30Z"
persona = "design engineer"
capability = "get lints for the W family of the catalog"
outcome_text = "I see actionable diagnostics with stable codes"
labels = ["creates:crates/cadetta-lint/src/rules/w.rs", "creates:tests/conformance/lints/W*/**"]
scope = ["crates/cadetta-lint/src/rules/w.rs", "tests/conformance/lints/W*/**"]

[[acceptance]]
text = "Given an expired waiver, when linted, then W001 error"
bound = false

[[acceptance]]
text = "Given a waiver whose target digest changed, when linted, then W002"
bound = false

[[acceptance]]
text = "Given a waiver of a fixed code, when linted, then W003"
bound = false

[[acceptance]]
text = "Given a waiver that suppresses nothing, when linted, then W004"
bound = false

[[acceptance]]
text = "Given a waiver path glob like `acme.parts::Bearing608.*`, when matched, then it suppresses one code on the matching paths (lints.md 2)"
bound = false

[[acceptance]]
text = "Given a waiver of a `require`, when linted, then W003; a require is discharged only by attestation (lints.md 2)"
bound = false

[[acceptance]]
text = "Given a waiver naming a fixed code or a require diagnostic, when linted, then W003 fires; a require is never suppressed (lints.md 2.1, 2.3; obligations.md 4)"
bound = false

[[acceptance]]
text = "Given waive ... for alt SLOT=ALT, when evaluated, then it scopes to that variant; W002 triggers when the lock waiver record is missing or digests differ (lints.md 2.1)"
bound = false

[[acceptance]]
text = 'Given #[allow(CODE, "because")], when linted, then it needs a reason, no expiry, no lock record, and W004 checks it (lints.md 2.2)'
bound = false

[[acceptance]]
text = "Given the rule, when shipped, then a must-fire and a must-not-fire fixture exist under tests/conformance/lints/<code>/"
bound = false
+++
