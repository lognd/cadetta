+++
id = "01M4YMHQ5BQW8T67PVT5ZFM9K6"
title = "CLI: implement ir-validate FILE and lint FILE conformance verbs"
type = "task"
category = "todo"
priority = "medium"
points = 3
reporter = "lognd"
created = "2026-10-15T01:59:07Z"
updated = "2026-10-15T02:18:19Z"
idempotency_key = "cli-ir-validate-lint-verbs"
scope = ["crates/cadetta-cli/src/**", "docs/spec/config-cli.md", "crates/cadetta-cli/Cargo.toml", "Cargo.lock", "crates/cadetta-cli/tests/ir_entries.rs", "changelog.d/01M4YMHQ5BQW8T67PVT5ZFM9K6.added.md"]

[[acceptance]]
text = "ir-validate and lint FILE verbs exist with the exit codes in config-cli.md 3.9"
bound = false
+++

Spec config-cli.md 3.9 (added by ~ZM6GZJ0) defines ir-validate and lint on IR JSON; neither verb exists in the CLI.
