+++
id = "01M4TRKDEST57137A1Y203JFR7"
title = "Waivers: allow attributes, closure digests, require paths and W005 in check"
type = "task"
category = "todo"
priority = "medium"
points = 5
reporter = "lognd"
created = "2026-10-13T13:52:59Z"
updated = "2026-10-15T01:54:49Z"
scope = ["crates/cadetta-cli/src/pipeline.rs,crates/cadetta-cli/src/cmd/check.rs,crates/cadetta-syntax/src/**,crates/cadetta-lint/src/**", "crates/cadetta-cli/src/universe.rs", "docs/spec/lints.md", "changelog.d/01M4TRKDEST57137A1Y203JFR7.*"]

[[acceptance]]
text = "#[allow] is parsed and applied; W002 fires on a changed closure; W003 fires for requires; W005 reported"
bound = true
+++

Follow-up of ~MC5WD33: check applies waive declarations only. #[allow] attributes are not parsed into Allow, Suppressions.closures and targets_closure are empty (W002 cannot fire), require_paths is empty (W003 for requires), W005 is not wired, waiver globs are not carried over renames.
