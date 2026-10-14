+++
id = "01M4XE4KFVQ24QF7QMXPBQGCFY"
title = "cadetta lock verify CLI verb with --format json (report and schema landed in 0DM3P8Y)"
type = "story"
category = "in-progress"
priority = "medium"
points = 2
reporter = "lognd"
created = "2026-10-14T14:47:52Z"
updated = "2026-10-14T14:52:25Z"
labels = ["creates:crates/cadetta-cli/src/cmd/lock.rs", "creates:changelog.d/01M4XE4KFVQ24QF7QMXPBQGCFY.added.md"]
scope = ["crates/cadetta-cli/src/cmd/mod.rs", "crates/cadetta-cli/src/lib.rs", "docs/spec/config-cli.md", "crates/cadetta-cli/src/cmd/lock.rs", "changelog.d/01M4XE4KFVQ24QF7QMXPBQGCFY.added.md"]

[[acceptance]]
text = "cadetta lock verify reads cadetta.lock, checks it against live obligation digests and configured roles, and exits 0 when clean, 1 on any finding, 2 on usage or unreadable lock"
bound = true

[[acceptance]]
text = "Format json emits the cadetta.lock/1 report from VerifyReport::to_json; text prints one line per finding or 'cadetta.lock: clean'"
bound = true

[[acceptance]]
text = "docs/spec/config-cli.md documents the verb and a changelog fragment exists"
bound = true
+++
