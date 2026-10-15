+++
id = "01M4TZY6AT7ZPFVRT8N50N0ZB6"
title = "Remove dead check_r004/check_r006 engine functions (superseded by check_drift and sync)"
type = "task"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T16:01:13Z"
updated = "2026-10-15T03:07:47Z"
scope = ["crates/cadetta-lint/src/**", "crates/cadetta-lint/src/rules/r.rs", "crates/cadetta-cli/src/cmd/sync.rs", "changelog.d/**", "docs/**"]

[[acceptance]]
text = "check_r004/check_r006, RebuildFailure, Replacement and their tests are removed; remaining R rules still pass"
bound = false
+++

R001-R003/R005 are wired via check_drift.rs (~3RK3GPH); R004 and R006 are raised by cmd/sync.rs and cadetta-adapter sync::R006. check_r004 and check_r006 (with RebuildFailure, Replacement) in rules/r.rs are unused duplicates: remove them and their tests, or adopt them in sync.rs if that removes duplication.
