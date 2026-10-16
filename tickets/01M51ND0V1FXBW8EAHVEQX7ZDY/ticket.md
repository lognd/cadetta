+++
id = "01M51ND0V1FXBW8EAHVEQX7ZDY"
title = "process rule that changes the argument kind is only logged, never an EVAL diagnostic"
type = "bug"
category = "in-progress"
priority = "medium"
points = 2
reporter = "lognd"
created = "2026-10-16T06:11:45Z"
updated = "2026-10-16T07:45:48Z"
scope = ["crates/cadetta-sema/src/process.rs", "crates/cadetta-cli/src/pipeline.rs", "tests/conformance/known-gaps.toml"]

[[acceptance]]
text = "A rule changing the argument kind yields an EVAL error at check time; language/process-rule-kind gap removed"
bound = true
+++

found while working ~EB0V9X7: spec 3.8 says a process rule must keep the argument's kind; sema only tracing-warns 'changes the kind' and emits no diagnostic. Fixture language/process-rule-kind is gapped until fixed.
