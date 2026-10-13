+++
id = "01M4SWN76NXYQCYDG7C1A8C2KG"
title = "Waiver obligation kind and lock waiver record"
type = "story"
category = "in-progress"
priority = "medium"
points = 3
parent = "01M4ST8EJWTGQM9CMVBJVBATJT"
reporter = "lognd"
created = "2026-10-13T05:44:38Z"
updated = "2026-10-13T14:11:18Z"
persona = "design engineer"
capability = "review waivers like everything else"
idempotency_key = "replan14:ob-waiver"
labels = ["creates:crates/cadetta-obligations/src/waiver.rs"]
scope = ["crates/cadetta-obligations/src/waiver.rs", "crates/cadetta-obligations/src/lib.rs"]

[[links]]
kind = "blocked-by"
target = "01M4SPWRDX98PHVNHPD14HYY8D"

[[links]]
kind = "blocked-by"
target = "01M4ST8EMQTXAGZAWGE2M5A4BR"

[[links]]
kind = "blocked-by"
target = "01M4ST8EPKBWRWHF51Y5NV3VK0"

[[acceptance]]
text = "Given a waiver, when graphed, then a waiver:<glob> obligation exists and a lock waiver record binds digests; changed digests give W002 (obligations.md 2, 3; lints.md 2.1)"
bound = false
+++
