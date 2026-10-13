+++
id = "01M4SPWS23PN7M3TKKW8SA8QRK"
title = "Emit policy: tolerance point and Q004 precision"
type = "story"
category = "todo"
priority = "medium"
points = 3
parent = "01M4SPWH74K9QQS72Z9YMTPD72"
reporter = "lognd"
created = "2026-10-13T04:03:55Z"
updated = "2026-10-13T04:03:55Z"
persona = "design engineer"
capability = "choose nominal, mid, mmc or lmc and see Q004 for unrepresentable values"
outcome_text = "emitters decide nothing and never round silently"
labels = ["creates:crates/cadetta-adapter/src/policy.rs"]
scope = ["crates/cadetta-adapter/src/policy.rs"]

[[acceptance]]
text = 'Given `point = "mid"`, when resolved, then the midpoint of the limits is selected'
bound = false

[[acceptance]]
text = "Given a rational not representable at declared precision, when converted, then Q004 is returned with the error"
bound = false
+++
