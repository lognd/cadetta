+++
id = "01M4SWN7A33MPRTACHTTNH3SKN"
title = "lock verify"
type = "story"
category = "in-progress"
priority = "medium"
points = 5
parent = "01M4ST8EJWTGQM9CMVBJVBATJT"
reporter = "lognd"
created = "2026-10-13T05:44:38Z"
updated = "2026-10-13T07:50:32Z"
persona = "design engineer"
capability = "catch merge-invalidated attestations"
idempotency_key = "replan14:ob-verify"
labels = ["creates:crates/cadetta-obligations/src/lock_verify.rs"]
scope = ["crates/cadetta-obligations/src/lock_verify.rs", "crates/cadetta-obligations/src/lib.rs"]

[[links]]
kind = "blocked-by"
target = "01M4SWN78DB7D7SPY3X8BHQT8Y"

[[acceptance]]
text = "Given a merged lock, when verify runs, then attestations invalidated by the merge are listed and exit is 1 (config-cli.md 3; obligations.md 3)"
bound = false
+++
