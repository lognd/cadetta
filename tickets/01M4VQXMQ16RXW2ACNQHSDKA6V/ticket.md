+++
id = "01M4VQXMQ16RXW2ACNQHSDKA6V"
title = "Conformance runner: executors for emit, emit-twice, emit-compare, impact-swap binding entries"
type = "task"
category = "in-progress"
priority = "medium"
points = 3
parent = "01M4STAPYJBMRZBEY8HJH6F6XS"
reporter = "lognd"
created = "2026-10-13T23:00:20Z"
updated = "2026-10-14T02:21:23Z"
scope = ["crates/cadetta-cli/tests/**"]

[[acceptance]]
text = "Each of these entries executes and is compared; their known-gaps.toml entries are removed."
bound = false
+++

found while working ~6BCVN48: kind=binding entries emit, emit-twice (2), emit-compare, impact-swap (1 each) in adapters/ have no executor.
