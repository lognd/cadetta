+++
id = "01M4SWN74DYM9HNKRXYFY92Q27"
title = "Definition versus instance level grouping"
type = "story"
category = "done"
outcome = "done"
priority = "medium"
points = 3
parent = "01M4ST8EJWTGQM9CMVBJVBATJT"
reporter = "lognd"
created = "2026-10-13T05:44:38Z"
updated = "2026-10-13T08:25:10Z"
persona = "design engineer"
capability = "attest once for identical instances"
idempotency_key = "replan14:ob-level"
labels = ["creates:crates/cadetta-obligations/src/level.rs"]
scope = ["crates/cadetta-obligations/src/level.rs", "crates/cadetta-obligations/src/lib.rs", "docs/spec/obligations.md"]

[[links]]
kind = "blocked-by"
target = "01M4ST8EMQTXAGZAWGE2M5A4BR"

[[links]]
kind = "blocked-by"
target = "01M4SWK9D2DYTDN1TW3013FBDV"

[[acceptance]]
text = "Given a closure reading no component parameter or interface slot, when targeted, then the obligation is at definition path; otherwise instance paths with identical argument closures form one obligation (obligations.md 2; ir.md 2)"
bound = true
+++
