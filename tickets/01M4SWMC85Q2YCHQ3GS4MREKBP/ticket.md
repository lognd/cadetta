+++
id = "01M4SWMC85Q2YCHQ3GS4MREKBP"
title = "Snapshot and atomic restore on regen failure"
type = "story"
category = "todo"
priority = "medium"
points = 5
parent = "01M4SPWH74K9QQS72Z9YMTPD72"
reporter = "lognd"
created = "2026-10-13T05:44:11Z"
updated = "2026-10-14T21:04:28Z"
persona = "design engineer"
capability = "never leave CAD half-pushed"
idempotency_key = "replan14:ad-atomic"
labels = ["creates:crates/cadetta-adapter/src/atomic.rs"]
scope = ["crates/cadetta-adapter/src/atomic.rs"]

[[links]]
kind = "blocked-by"
target = "01M4SWMC5P00K0B1SZ9JPTJW8W"

[[acceptance]]
text = "Given a push whose regen fails (R004), when sync finishes, then the host restores the pre-push snapshot so no CAD file stays half-written (adapters.md 2.3)"
bound = true

[[acceptance]]
text = "Given a read-back that differs from what was pushed, when recording, then a synced record is written only for locators whose read-back matches (adapters.md 2.3)"
bound = true
+++
