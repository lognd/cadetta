+++
id = "01M4SWKAH8CP5VJYR93A9M9EPG"
title = "Lint family R007-R013 adapter and binding rules"
type = "story"
category = "done"
outcome = "done"
priority = "medium"
points = 5
parent = "01M4SPWH0ZE4G7KXRDA5M71GPS"
reporter = "lognd"
created = "2026-10-13T05:43:36Z"
updated = "2026-10-14T21:02:34Z"
persona = "design engineer"
capability = "see why the CAD tool refused a push"
idempotency_key = "replan14:lint-r2"
labels = ["creates:crates/cadetta-lint/src/rules/r_adapter.rs", "creates:tests/conformance/lints/R0[01]*/**"]
scope = ["crates/cadetta-lint/src/rules/r_adapter.rs", "tests/conformance/lints/R0[01]*/**", "docs/spec/lints.md", "crates/cadetta-lint/src/rules/mod.rs", "changelog.d/01M4SWKAH8CP5VJYR93A9M9EPG.added.md"]

[[links]]
kind = "blocked-by"
target = "01M4SPWQM289M7Q58CSS6F029F"

[[links]]
kind = "blocked-by"
target = "01M4SPWRAWXJTWT4G08578RNQM"

[[acceptance]]
text = "Given R004 and R008-R010 from an adapter response, when linted, then they pass through unchanged; R008 is error on sync and info on check (lints.md 3 R)"
bound = true

[[acceptance]]
text = "Given R011 (assembly holds a file other than the slot primary's), R012 ([*] binding over differing values) and R013 (one native name from two paths, fixed), when linted, then each fires (lints.md 3 R; adapters.md 3)"
bound = true

[[acceptance]]
text = "Given R007, when a bound coordinate's sketch is not on its datum plane or is not measured from the sketch origin, then a warning fires (lints.md 3 R; geometry.md 6)"
bound = true
+++
