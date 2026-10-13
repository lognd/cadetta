+++
id = "01M4SWN74DYM9HNKRXYFY92Q27"
title = "Definition versus instance level grouping"
type = "story"
category = "todo"
priority = "medium"
points = 3
parent = "01M4ST8EJWTGQM9CMVBJVBATJT"
reporter = "lognd"
created = "2026-10-13T05:44:38Z"
updated = "2026-10-13T05:44:43Z"
persona = "design engineer"
capability = "attest once for identical instances"
idempotency_key = "replan14:ob-level"
labels = ["creates:crates/cadetta-obligations/src/level.rs"]
scope = ["crates/cadetta-obligations/src/level.rs"]

[[links]]
kind = "blocked-by"
target = "01M4ST8EMQTXAGZAWGE2M5A4BR"

[[acceptance]]
text = "Given a closure reading no component parameter or interface slot, when targeted, then the obligation is at definition path; otherwise instance paths with identical argument closures form one obligation (obligations.md 2; ir.md 2)"
bound = false
+++
