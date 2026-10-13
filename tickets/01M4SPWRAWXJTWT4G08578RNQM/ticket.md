+++
id = "01M4SPWRAWXJTWT4G08578RNQM"
title = "Lint family Drift rules R001-R006 (needs observed IR)"
type = "story"
category = "done"
outcome = "done"
priority = "medium"
points = 8
parent = "01M4SPWH0ZE4G7KXRDA5M71GPS"
reporter = "lognd"
created = "2026-10-13T04:03:54Z"
updated = "2026-10-13T16:01:13Z"
persona = "design engineer"
capability = "get lints for the R family of the catalog"
outcome_text = "I see actionable diagnostics with stable codes"
labels = ["creates:crates/cadetta-lint/src/rules/r.rs", "creates:tests/conformance/lints/R*/**"]
scope = ["crates/cadetta-lint/src/rules/r.rs", "tests/conformance/lints/R*/**", "crates/cadetta-lint/src/rules/mod.rs"]

[[acceptance]]
text = "Given an observed value differing from design at a bound path, when linted, then R001 names which side moved since last sync"
bound = true

[[acceptance]]
text = "Given a native parameter with no binding, when linted, then R002 warning"
bound = true

[[acceptance]]
text = "Given a bound path absent natively, when linted, then R003 error"
bound = true

[[acceptance]]
text = "Given a binding matching no IR path and no `was` path, when linted, then R005 warning (adapters.md 3)"
bound = true

[[acceptance]]
text = "Given a drive adapter reporting a rebuild failure, when linted, then R004 error is surfaced (adapters.md 1)"
bound = true

[[acceptance]]
text = "Given drift, when R001 is reported, then it compares against the emitted point, not the nominal (adapters.md 3a)"
bound = true

[[acceptance]]
text = "Given the three-way compare, when R001 fires, then it states cad moved, design moved or both moved and shows base, ours and theirs (adapters.md 2; lints.md 3 R)"
bound = true

[[acceptance]]
text = "Given a push that replaces a native expression, when linted, then R006 is a warning (not info) listing the old expression, and also fires from an adopted record (lints.md 3 R; adapters.md 2.1, 7.4)"
bound = true
+++
