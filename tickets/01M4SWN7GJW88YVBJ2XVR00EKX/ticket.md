+++
id = "01M4SWN7GJW88YVBJ2XVR00EKX"
title = "impact --since and --worktree"
type = "story"
category = "todo"
priority = "medium"
points = 8
parent = "01M4ST8EJWTGQM9CMVBJVBATJT"
reporter = "lognd"
created = "2026-10-13T05:44:39Z"
updated = "2026-10-13T05:44:39Z"
persona = "design engineer"
capability = "review a change before merging"
idempotency_key = "replan14:ob-since"
labels = ["creates:crates/cadetta-obligations/src/impact_since.rs"]
scope = ["crates/cadetta-obligations/src/impact_since.rs"]

[[acceptance]]
text = "Given two evaluations (git REV or worktree), when diffed, then changed nodes, stale obligations, flipped verdicts and CAD files to re-sync are reported (obligations.md 5; config-cli.md 3)"
bound = false
+++
