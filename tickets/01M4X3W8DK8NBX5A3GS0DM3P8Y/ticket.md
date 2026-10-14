+++
id = "01M4X3W8DK8NBX5A3GS0DM3P8Y"
title = "lock verify --format json report and docs/schemas/lock.json"
type = "chore"
category = "in-progress"
priority = "low"
points = 3
reporter = "Claude"
created = "2026-10-14T11:48:32Z"
updated = "2026-10-14T14:46:54Z"
labels = ["creates:docs/schemas/lock.json", "creates:changelog.d/01M4X3W8DK8NBX5A3GS0DM3P8Y.added.md"]
scope = ["crates/cadetta-obligations/src/lock_verify.rs", "crates/cadetta-obligations/Cargo.toml", "docs/spec/config-cli.md", "docs/spec/obligations.md", "docs/schemas/lock.json", "changelog.d/01M4X3W8DK8NBX5A3GS0DM3P8Y.added.md"]

[[acceptance]]
text = "lock verify report serializes to JSON (cadetta.lock/1) with clean, exit_code and typed findings"
bound = true

[[acceptance]]
text = "docs/schemas/lock.json exists and the report validates against it in a test"
bound = true

[[acceptance]]
text = "spec docs and changelog fragment describe the report"
bound = true
+++
