+++
id = "01M4STCCVN188CD3VRZC374QTM"
title = "Lint U001 unused parameter"
type = "story"
category = "in-progress"
priority = "medium"
points = 2
parent = "01M4SPWH0ZE4G7KXRDA5M71GPS"
reporter = "lognd"
created = "2026-10-13T05:04:52Z"
updated = "2026-10-13T09:09:50Z"
persona = "design engineer"
capability = "find parameters nothing reads"
idempotency_key = "replan13:Lint U001 unused parameter"
labels = ["creates:crates/cadetta-lint/src/rules/unused.rs"]
scope = ["crates/cadetta-lint/src/rules/unused.rs"]

[[acceptance]]
text = "Given a model parameter read by nothing and bound to nothing, when linted, then U001 fires (lints.md U)"
bound = false

[[acceptance]]
text = "Given one read or bound, when linted, then U001 does not fire (lints.md U)"
bound = false
+++
