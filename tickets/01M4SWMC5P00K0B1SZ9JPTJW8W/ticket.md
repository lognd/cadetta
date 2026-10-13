+++
id = "01M4SWMC5P00K0B1SZ9JPTJW8W"
title = "Sync orchestration, dry-run and scoping"
type = "story"
category = "in-progress"
priority = "medium"
points = 8
parent = "01M4SPWH74K9QQS72Z9YMTPD72"
reporter = "lognd"
created = "2026-10-13T05:44:11Z"
updated = "2026-10-13T14:20:02Z"
persona = "design engineer"
capability = "keep CAD and design in step"
idempotency_key = "replan14:ad-sync"
labels = ["creates:crates/cadetta-adapter/src/sync.rs"]
scope = ["crates/cadetta-adapter/src/sync.rs", "crates/cadetta-adapter/src/lib.rs"]

[[links]]
kind = "blocked-by"
target = "01M4SPWRS0CDJT4GYC92AB1HCD"

[[links]]
kind = "blocked-by"
target = "01M4SWMBMD210J9ZWQEF95V4RW"

[[acceptance]]
text = "Given an unchanged design and CAD, when sync runs twice, then the second run pushes nothing and writes an identical lock (adapters.md 2.1)"
bound = true

[[acceptance]]
text = "Given --dry-run, --only GLOB, --no-regen and ADAPTER-ID, when sync runs, then they scope or suppress as config-cli.md 3.1 states; --dry-run writes nothing and lists each replaced native expression (R006)"
bound = true

[[acceptance]]
text = "Given --no-regen, when pushing, then the lock is not updated (config-cli.md 3.1)"
bound = true
+++
