+++
id = "01M4YM3CDA6F41F84EWPY1103W"
title = "Lint engine resets diagnostic severity, undoing D001 downgrade"
type = "bug"
category = "todo"
priority = "low"
points = 2
reporter = "lognd"
created = "2026-10-15T01:51:17Z"
updated = "2026-10-15T01:51:17Z"
idempotency_key = "followup-xn48psv-engine-severity"
scope = ["crates/cadetta-lint", "crates/cadetta-cli/src/cmd/check.rs"]

[[acceptance]]
text = "Engine preserves a rule-set severity; check.rs workaround removed; a test in cadetta-lint proves a downgraded D001 stays info"
bound = false
+++

check.rs works around the engine resetting every diagnostic to its catalog severity (D001 downgrade below d001_min_digits). Move the fix into crates/cadetta-lint/src/engine.rs and drop the workaround.
