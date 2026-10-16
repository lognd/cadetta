+++
id = "01M5253XXZKAAXCMDEBMJX2SB1"
title = "cadetta update exits 2 on E012/E013; spec says exit 1"
type = "bug"
category = "in-progress"
priority = "medium"
points = 1
reporter = "lognd"
created = "2026-10-16T10:46:24Z"
updated = "2026-10-16T11:08:08Z"
scope = ["crates/cadetta-cli/src/cmd/update.rs"]
+++

Found in ~VD4Q7XX. run_update calls Project::open, which resolves the closure and returns E012/E013 as ProjectError -> EXIT_USAGE (2). config-cli.md and UpdateError::exit_code say 1. Fix: map dep diagnostics from open to exit 1. Then remove the two known-gaps entries packages/update-e012-lock-untouched and packages/update-e013-lock-untouched.
