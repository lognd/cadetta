+++
id = "01M4VQXMBWSFHWEVGFBBC7S5KT"
title = "Conformance runner: executors for binding sync and push-plan entries"
type = "task"
category = "todo"
priority = "medium"
points = 5
parent = "01M4STAPYJBMRZBEY8HJH6F6XS"
reporter = "lognd"
created = "2026-10-13T23:00:20Z"
updated = "2026-10-15T08:12:43Z"
scope = ["crates/cadetta-cli/tests/**", "tests/conformance/known-gaps.toml", "tests/conformance/README.md"]

[[acceptance]]
text = "Every sync and push-plan fixture executes and is compared; their known-gaps.toml entries are removed."
bound = false
+++

found while working ~6BCVN48: kind=binding entry=sync (adapters 32, emitters 7) and push-plan (9) have no executor; cadetta_adapter::sync dry-run is available (the classify executor uses it).
