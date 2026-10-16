+++
id = "01M4VQXMH7YHJT18D9NJBHTKF3"
title = "Conformance runner: executor for binding import entries"
type = "task"
category = "done"
outcome = "done"
priority = "medium"
points = 5
parent = "01M4STAPYJBMRZBEY8HJH6F6XS"
reporter = "lognd"
created = "2026-10-13T23:00:20Z"
updated = "2026-10-16T18:50:39Z"
scope = ["crates/cadetta-cli/tests/**", "tests/conformance/known-gaps.toml", "tests/conformance/README.md"]

[[acceptance]]
text = "Every import fixture executes and is compared; their known-gaps.toml entries are removed."
bound = true
+++

found while working ~6BCVN48: kind=binding entry=import (adapters, 31 fixtures) has no executor; use cadetta_adapter::import (see ~CN4M56T, ~Z1PNF3W).
