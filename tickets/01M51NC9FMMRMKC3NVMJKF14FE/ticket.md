+++
id = "01M51NC9FMMRMKC3NVMJKF14FE"
title = "Binding conformance red: duplicate E002 and false D002 on fit hole source"
type = "bug"
category = "todo"
priority = "medium"
points = 3
reporter = "lognd"
created = "2026-10-16T06:11:21Z"
updated = "2026-10-16T06:11:21Z"
scope = ["bindings/python/**", "crates/cadetta-cli/src/pipeline.rs", "crates/cadetta-lint/src/rules/d.rs", "crates/cadetta-cli/tests/lint_gaps_d001_d002.rs"]

[[acceptance]]
text = "binding pytest test_conformance passes for the 20 failing fixtures; no duplicate E002; D002 not raised on the authored fit hole source literal"
bound = false
+++

Python test is correct (closed-list parity with the Rust runner). Root causes in CLI/lint: (1) lift_resolve_error sets path Some("") defeating twin dedup (6c9d5cad); fix restores base path. (2) check_d002 matches fit.hole to new derived accessor members (b0c8969a) so the authored source literal is flagged; fix excludes derived members as the literal's own.
