+++
id = "01M4TZKF0EKQ5E4N9KTNGEACDP"
title = "solve --format json report and solve-report.json schema"
type = "bug"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T15:55:21Z"
updated = "2026-10-15T01:51:04Z"
scope = ["crates/cadetta-cli/src/cmd/solve.rs", "docs/schemas/*.json"]

[[acceptance]]
text = "Given solvers, when solve --format json runs, then a report validating against solve-report.json lists each solver as solved, fresh or failed with its code"
bound = true
+++

found while working ~VN1YQYC: solve prints text only; lock and sync commands do not exist yet, so lock.json and sync.json belong to those command tickets.
