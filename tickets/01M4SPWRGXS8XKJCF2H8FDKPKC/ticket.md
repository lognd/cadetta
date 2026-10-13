+++
id = "01M4SPWRGXS8XKJCF2H8FDKPKC"
title = "Adapter trait, capabilities and error type"
type = "story"
category = "todo"
priority = "medium"
points = 3
parent = "01M4SPWH74K9QQS72Z9YMTPD72"
reporter = "lognd"
created = "2026-10-13T04:03:54Z"
updated = "2026-10-13T05:01:25Z"
persona = "cadetta contributor"
capability = "implement an adapter declaring emit, read and drive capabilities"
outcome_text = "new CAD targets never touch lints"
labels = ["creates:crates/cadetta-adapter/src/lib.rs"]
scope = ["crates/cadetta-adapter/src/lib.rs"]

[[links]]
kind = "blocks"
target = "01M4SPWRS0CDJT4GYC92AB1HCD"

[[links]]
kind = "blocks"
target = "01M4SPWRYRBTGD67WTG7D60E59"

[[links]]
kind = "blocks"
target = "01M4SPWS23PN7M3TKKW8SA8QRK"

[[acceptance]]
text = "Given an adapter without drive, when sync is requested, then it degrades to emit plus instructions"
bound = false

[[acceptance]]
text = "Given an adapter result, when failing, then it returns a Result with a diagnostic, not a panic"
bound = false

[[acceptance]]
text = "Given a drive-capable adapter, when regeneration fails, then rebuild errors are reported as R004 diagnostics (adapters.md 1)"
bound = false
+++
