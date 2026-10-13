+++
id = "01M4SWMBMD210J9ZWQEF95V4RW"
title = "Three-way sync classifier"
type = "story"
category = "todo"
priority = "medium"
points = 5
parent = "01M4SPWH74K9QQS72Z9YMTPD72"
reporter = "lognd"
created = "2026-10-13T05:44:10Z"
updated = "2026-10-13T05:44:10Z"
persona = "design engineer"
capability = "know what a sync will do"
idempotency_key = "replan14:ad-classify"
labels = ["creates:crates/cadetta-adapter/src/classify.rs"]
scope = ["crates/cadetta-adapter/src/classify.rs"]

[[acceptance]]
text = "Given base (lock), design and CAD values, when classified, then each of push, skip, converged, drift (R001) and conflict follows the adapters.md 2 table row for row"
bound = false

[[acceptance]]
text = "Given the emit-point header, when computing no-op, then it is excluded; with --force-design a CAD-moved value is pushed instead of R001 (adapters.md 2)"
bound = false
+++
