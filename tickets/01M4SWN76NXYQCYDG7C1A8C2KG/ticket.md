+++
id = "01M4SWN76NXYQCYDG7C1A8C2KG"
title = "Waiver obligation kind and lock waiver record"
type = "story"
category = "todo"
priority = "medium"
points = 3
parent = "01M4ST8EJWTGQM9CMVBJVBATJT"
reporter = "lognd"
created = "2026-10-13T05:44:38Z"
updated = "2026-10-13T05:44:38Z"
persona = "design engineer"
capability = "review waivers like everything else"
idempotency_key = "replan14:ob-waiver"
labels = ["creates:crates/cadetta-obligations/src/waiver.rs"]
scope = ["crates/cadetta-obligations/src/waiver.rs"]

[[acceptance]]
text = "Given a waiver, when graphed, then a waiver:<glob> obligation exists and a lock waiver record binds digests; changed digests give W002 (obligations.md 2, 3; lints.md 2.1)"
bound = false
+++
