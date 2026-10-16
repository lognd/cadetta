+++
id = "01M4ZCEY9XVFNZNWWXA6K6E7M4"
title = "Slots fixtures: activate an alternate and expose active_alternate"
type = "task"
category = "in-progress"
priority = "medium"
points = 3
reporter = "lognd"
created = "2026-10-15T08:57:02Z"
updated = "2026-10-16T18:58:11Z"
scope = ["crates/cadetta-cli/src/cmd/emit.rs", "crates/cadetta-cli/src/cmd/sync.rs", "crates/cadetta-cli/tests/conformance/exec.rs", "crates/cadetta-cli/src/universe.rs", "tests/conformance/adapters/slots-for-selects-active-locators.*/**", "tests/conformance/adapters/slots-swap-changes-active-set.*/**", "tests/conformance/known-gaps.toml", "docs/schemas/emit.json", "docs/spec/adapters.md", "docs/spec/emitters/sw-equations.md", "docs/spec/config-cli.md"]
+++

slots-for-selects-active-locators and slots-swap-changes-active-set expect key active_alternate, but no source activates an alternate, the CLI exposes no active alternate, and the swap fixture's must-fire and must-not-fire dirs are byte-identical while expecting opposite output. Needs spec/fixture decision plus emit support (for-alternate locators are always skipped today). Split from ~SK1B9S3.
