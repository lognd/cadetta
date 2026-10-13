+++
id = "01M4ST7B3CYJNHGNCCQ3EYW9WC"
title = "Unit assumption at read"
type = "story"
category = "todo"
priority = "medium"
points = 3
parent = "01M4SPWH74K9QQS72Z9YMTPD72"
reporter = "lognd"
created = "2026-10-13T05:02:06Z"
updated = "2026-10-13T05:02:06Z"
persona = "design engineer"
capability = "read native values that carry no unit"
idempotency_key = "replan13:Unit assumption at read"
labels = ["creates:crates/cadetta-adapter/src/read_units.rs"]
scope = ["crates/cadetta-adapter/src/read_units.rs"]

[[acceptance]]
text = "Given a native value without a unit, when read, then it takes the adapter declared native unit and is recorded as unit_assumed (adapters.md 3)"
bound = false

[[acceptance]]
text = "Given a format with no unit information and no `--assume-unit`, when read, then the value is read with exact:false and a reason (adapters.md 3)"
bound = false

[[acceptance]]
text = "Given any read, when a value is unitless, then Q001 is never raised; Q001 is a source error (adapters.md 3)"
bound = false
+++
