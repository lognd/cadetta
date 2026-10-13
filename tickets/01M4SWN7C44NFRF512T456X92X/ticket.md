+++
id = "01M4SWN7C44NFRF512T456X92X"
title = "lock tidy"
type = "story"
category = "todo"
priority = "medium"
points = 3
parent = "01M4ST8EJWTGQM9CMVBJVBATJT"
reporter = "lognd"
created = "2026-10-13T05:44:39Z"
updated = "2026-10-13T05:44:44Z"
persona = "design engineer"
capability = "keep the lock small"
idempotency_key = "replan14:ob-tidy"
labels = ["creates:crates/cadetta-obligations/src/lock_tidy.rs"]
scope = ["crates/cadetta-obligations/src/lock_tidy.rs"]

[[links]]
kind = "blocked-by"
target = "01M4ST8EPKBWRWHF51Y5NV3VK0"

[[acceptance]]
text = "Given superseded records, when tidy runs, then they are dropped and live records are untouched (obligations.md 3; config-cli.md 3)"
bound = false
+++
