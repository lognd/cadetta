+++
id = "01M4Z9WQPNGBM5J14R2CDJGJMG"
title = "Conformance: sync executor for binding sync fixtures"
type = "task"
category = "in-progress"
priority = "medium"
points = 5
reporter = "lognd"
created = "2026-10-15T08:12:08Z"
updated = "2026-10-15T08:15:56Z"
scope = ["crates/cadetta-cli/tests/**", "tests/conformance/known-gaps.toml", "tests/conformance/README.md"]

[[acceptance]]
text = "Every binding sync fixture executes and is compared; their known-gaps.toml entries are removed."
bound = false
+++

found while working ~BC7S5KT: sync fixtures (adapters/ and emitters/*) still lack an executor; build on sync::run plus protocol.rs scripted adapters.
