+++
id = "01M4SWMBMD210J9ZWQEF95V4RW"
title = "Three-way sync classifier"
type = "story"
category = "done"
outcome = "done"
priority = "medium"
points = 5
parent = "01M4SPWH74K9QQS72Z9YMTPD72"
reporter = "lognd"
created = "2026-10-13T05:44:10Z"
updated = "2026-10-13T09:33:34Z"
persona = "design engineer"
capability = "know what a sync will do"
idempotency_key = "replan14:ad-classify"
labels = ["creates:crates/cadetta-adapter/src/classify.rs"]
scope = ["crates/cadetta-adapter/src/classify.rs", "crates/cadetta-adapter/src/lib.rs"]

[[links]]
kind = "blocked-by"
target = "01M4SPWRGXS8XKJCF2H8FDKPKC"

[[links]]
kind = "blocked-by"
target = "01M4ST8EPKBWRWHF51Y5NV3VK0"

[[acceptance]]
text = "Given base (lock), design and CAD values, when classified, then each of push, skip, converged, drift (R001) and conflict follows the adapters.md 2 table row for row"
bound = true

[[acceptance]]
text = "Given the emit-point header, when computing no-op, then it is excluded; with --force-design a CAD-moved value is pushed instead of R001 (adapters.md 2)"
bound = true
+++
