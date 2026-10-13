+++
id = "01M4SPWRDX98PHVNHPD14HYY8D"
title = "Lint family Waiver rules W001-W004"
type = "story"
category = "in-progress"
priority = "medium"
points = 3
parent = "01M4SPWH0ZE4G7KXRDA5M71GPS"
reporter = "lognd"
created = "2026-10-13T04:03:54Z"
updated = "2026-10-13T13:16:55Z"
persona = "design engineer"
capability = "get lints for the W family of the catalog"
outcome_text = "I see actionable diagnostics with stable codes"
labels = ["creates:crates/cadetta-lint/src/rules/w.rs", "creates:tests/conformance/lints/W*/**"]
scope = ["crates/cadetta-lint/src/rules/w.rs", "tests/conformance/lints/W*/**"]

[[acceptance]]
text = "Given an expired waiver, when linted, then W001 error"
bound = true

[[acceptance]]
text = "Given a waiver of a fixed code, when linted, then W003"
bound = true

[[acceptance]]
text = "Given a waiver that suppresses nothing, when linted, then W004"
bound = true

[[acceptance]]
text = "Given a waiver path glob like `acme.parts::Bearing608.*`, when matched, then it suppresses one code on the matching paths (lints.md 2)"
bound = true

[[acceptance]]
text = "Given a waiver of a `require`, when linted, then W003; a require is discharged only by attestation (lints.md 2)"
bound = true

[[acceptance]]
text = "Given a waiver naming a fixed code or a require diagnostic, when linted, then W003 fires; a require is never suppressed (lints.md 2.1, 2.3; obligations.md 4)"
bound = true

[[acceptance]]
text = 'Given #[allow(CODE, "because")], when linted, then it needs a reason, no expiry, no lock record, and W004 checks it (lints.md 2.2)'
bound = true

[[acceptance]]
text = "Given a waiver with a lock waiver record whose target closure digest has since changed, when linted, then W002 fires and the suppressed diagnostic reappears until re-reviewed with `cadetta ack waiver:GLOB` (lints.md 2.1)"
bound = true

[[acceptance]]
text = "Given a waiver never reviewed (no lock record), when linted, then it still suppresses, its review is an open waiver obligation O002, and W002 does not fire; W002 fires only when a recorded review is stale (lints.md 2.1)"
bound = true

[[acceptance]]
text = "Given waive ... for alt SLOT=ALT, when evaluated, then it scopes to that variant, with the same O002 and W002 states (lints.md 2.1)"
bound = false
+++
