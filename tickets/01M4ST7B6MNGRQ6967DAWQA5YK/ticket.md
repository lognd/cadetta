+++
id = "01M4ST7B6MNGRQ6967DAWQA5YK"
title = "Adapter contract test harness"
type = "story"
category = "in-progress"
priority = "medium"
points = 3
parent = "01M4SPWH74K9QQS72Z9YMTPD72"
reporter = "lognd"
created = "2026-10-13T05:02:07Z"
updated = "2026-10-13T09:26:32Z"
persona = "adapter author"
capability = "prove an adapter meets the five contract rules"
idempotency_key = "replan13:Adapter contract test harness"
labels = ["creates:crates/cadetta-adapter/src/testing.rs"]
scope = ["crates/cadetta-adapter/src/testing.rs"]

[[acceptance]]
text = "Given an adapter, when emit runs twice, then the bytes are identical (adapters.md 4 rule 2)"
bound = false

[[acceptance]]
text = "Given a read-capable adapter, when emit then read run, then IR values compare equal (round trip) (adapters.md 4 rule 2)"
bound = false

[[acceptance]]
text = "Given a value not representable at the declared precision, when emitted, then Q004 with the error is returned, never silent rounding (adapters.md 4 rule 3)"
bound = false

[[acceptance]]
text = "Given a value the adapter cannot extract faithfully, when read, then exact:false with a reason, not a guess (adapters.md 4 rule 4)"
bound = false

[[acceptance]]
text = "Given any adapter, when run, then it never writes into .cdt source (adapters.md 4 rule 5)"
bound = false
+++
