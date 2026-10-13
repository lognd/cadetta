+++
id = "01M4SPWS23PN7M3TKKW8SA8QRK"
title = "Emit policy: tolerance point and Q004 precision"
type = "story"
category = "in-progress"
priority = "medium"
points = 3
parent = "01M4SPWH74K9QQS72Z9YMTPD72"
reporter = "lognd"
created = "2026-10-13T04:03:55Z"
updated = "2026-10-13T09:24:18Z"
persona = "design engineer"
capability = "choose nominal, mid, mmc or lmc and see Q004 for unrepresentable values"
outcome_text = "emitters decide nothing and never round silently"
labels = ["creates:crates/cadetta-adapter/src/policy.rs"]
scope = ["crates/cadetta-adapter/src/policy.rs"]

[[links]]
kind = "blocks"
target = "01M4SPWS56AF9GE891GAM5QSWX"

[[links]]
kind = "blocks"
target = "01M4SPWS7TFEF86PHX2W33Q6YX"

[[links]]
kind = "blocks"
target = "01M4SPWSABPXKK62B5V0SMJ8FD"

[[acceptance]]
text = 'Given `point = "mid"`, when resolved, then the midpoint of the limits is selected'
bound = true

[[acceptance]]
text = "Given a rational not representable at declared precision, when converted, then Q004 is returned with the error"
bound = false

[[acceptance]]
text = "Given `mmc` or `lmc` on a parameter with no role, when resolved, then an error is returned (quantities.md 5.6)"
bound = false

[[acceptance]]
text = "Given `#[emit(mid)]` on a fit side and a pure alias of it, when resolved, then the alias is emitted at mid too (quantities.md 5.6)"
bound = false

[[acceptance]]
text = "Given emit policy, when a request is built, then the tolerance point and unit are decided upstream and arrive in the request (adapters.md 4 rule 1)"
bound = false

[[acceptance]]
text = "Given emit point, when mid, mmc or lmc, then the emitted value follows the policy, and a changed emit attribute re-pushes via the sync record (ir.md 4)"
bound = false
+++
