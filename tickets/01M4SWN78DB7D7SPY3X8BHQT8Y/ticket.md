+++
id = "01M4SWN78DB7D7SPY3X8BHQT8Y"
title = "Lock merge driver"
type = "story"
category = "in-progress"
priority = "medium"
points = 5
parent = "01M4ST8EJWTGQM9CMVBJVBATJT"
reporter = "lognd"
created = "2026-10-13T05:44:38Z"
updated = "2026-10-13T07:37:58Z"
persona = "design engineer"
capability = "merge lock files without conflict"
idempotency_key = "replan14:ob-merge"
labels = ["creates:crates/cadetta-obligations/src/lock_merge.rs"]
scope = ["crates/cadetta-obligations/src/lock_merge.rs"]

[[links]]
kind = "blocked-by"
target = "01M4ST8EPKBWRWHF51Y5NV3VK0"

[[acceptance]]
text = "Given two branches adding records, when merged by cadetta lock merge, then records are unioned in sorted order with the tie rule and no textual conflicts (obligations.md 3, 6; config-cli.md 6)"
bound = false
+++
