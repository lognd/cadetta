+++
id = "01M4YRD0AKPB6JF7X2D1PKQ63N"
title = "Conformance: implement the compare executor (digest rules, schema classify)"
type = "task"
category = "in-progress"
priority = "medium"
points = 3
reporter = "lognd"
created = "2026-10-15T03:06:27Z"
updated = "2026-10-15T18:08:19Z"
idempotency_key = "compare-executor"
scope = ["crates/cadetta-cli/tests/conformance/**", "tests/conformance/**", "changelog.d/**", "docs/**"]

[[acceptance]]
text = "exec.rs runs compare fixtures for digest and classify and at least one fixture per entry passes"
bound = false
+++

Follow-up to ~MFYXJ4V: README specifies kind compare (entry digest, entry classify) but exec.rs has no executor, so such fixtures fail with no executor. Implement it and add fixtures for ir.md 4 digest rules and ir.md 5 classify.
