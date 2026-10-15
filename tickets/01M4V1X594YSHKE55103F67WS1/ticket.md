+++
id = "01M4V1X594YSHKE55103F67WS1"
title = "adapters.md 3: invalid bindings file or locator behavior unspecified and binary diverges"
type = "bug"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T16:35:36Z"
updated = "2026-10-15T02:53:19Z"
scope = ["docs/spec/adapters.md", "crates/cadetta-cli/src/cmd/emit.rs", "crates/cadetta-cli/src/cmd/sync.rs", "tests/conformance/known-gaps.toml", "tests/conformance/adapters/README.md", "changelog.d/01M4V1X594YSHKE55103F67WS1.fixed.md"]

[[acceptance]]
text = "Invalid bindings file or locator makes emit exit 2 naming the file and key, with no diagnostic code"
bound = false
+++

Spec decision recorded: invalid bindings file or locator is a configuration error, exit 2, names path/key, no diagnostic code. Binary emits R005 with exit 0. found while working ~YNT9TWR
