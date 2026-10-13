+++
id = "01M4SWN7RHS0FP5TFJC0WVCRM0"
title = "Require rule and O003/W003 interplay"
type = "story"
category = "todo"
priority = "medium"
points = 3
parent = "01M4ST8EJWTGQM9CMVBJVBATJT"
reporter = "lognd"
created = "2026-10-13T05:44:39Z"
updated = "2026-10-13T05:44:44Z"
persona = "design engineer"
capability = "keep requires unwaivable"
idempotency_key = "replan14:ob-require"
labels = ["creates:crates/cadetta-obligations/src/require.rs"]
scope = ["crates/cadetta-obligations/src/require.rs"]

[[links]]
kind = "blocked-by"
target = "01M4STCCZ58C875BTMQ9E54BMT"

[[acceptance]]
text = "Given a require, when evaluated, then O003 is never suppressed and a waiver naming it fires W003 (obligations.md 4; lints.md 2.3)"
bound = false
+++
