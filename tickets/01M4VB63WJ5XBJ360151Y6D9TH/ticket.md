+++
id = "01M4VB63WJ5XBJ360151Y6D9TH"
title = "Pure per-path mangling; R013 without module qualification"
type = "story"
category = "in-progress"
priority = "medium"
points = 2
parent = "01M4VB1MJC8NTMD6AAZR5Y5J08"
reporter = "lognd"
created = "2026-10-13T19:17:47Z"
updated = "2026-10-13T20:47:20Z"
persona = "design engineer"
capability = "rely on a created native name never changing when unrelated paths are added"
outcome_text = "links made in CAD survive every edit"
idempotency_key = "d15:pure-mangle"
scope = ["crates/cadetta-adapter/src/bindings.rs", "tests/conformance/adapters/**", "tests/conformance/bindings/**"]

[[acceptance]]
text = "Given any set of paths, when names are created, then each name is mangle(path, prefix) alone, including datum and placement op paths (adapters.md 3.1)"
bound = true

[[acceptance]]
text = "Given two in-scope paths with one created name, when emitting, then R013 before any write, and an explicit binding of one removes the collision (adapters.md 3.1)"
bound = true
+++
