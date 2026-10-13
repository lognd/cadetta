+++
id = "01M4ST7A42Q225NZMARTM5V6J5"
title = "Accessors, roles and bounds"
type = "story"
category = "in-progress"
priority = "medium"
points = 5
parent = "01M4SPWFZP4041AN6YQE08X2AN"
reporter = "lognd"
created = "2026-10-13T05:02:05Z"
updated = "2026-10-13T06:42:05Z"
persona = "design engineer"
capability = "read nominal, min, max, mid, mmc and lmc of any quantity, give it a hole or shaft role, and state one- or two-sided bounds"
idempotency_key = "replan13:Accessors, roles and bounds"
labels = ["creates:crates/cadetta-units/src/{accessors,bound,role}.rs"]
scope = ["crates/cadetta-units/src/{accessors,bound,role}.rs"]

[[acceptance]]
text = "Given an untoleranced quantity, when `.nominal .min .max .mid` are read, then all equal the value (quantities.md 5.2)"
bound = false

[[acceptance]]
text = "Given `12mm [+0.018mm -0]`, when accessors are read, then each returns an untoleranced quantity (quantities.md 5.2)"
bound = false

[[acceptance]]
text = "Given a role from an ISO class letter case, a fit side or `#[role(hole)]`, when `.mmc` and `.lmc` are read, then they follow the role; without a role they are an error (quantities.md 5.2, 5.6)"
bound = false

[[acceptance]]
text = "Given a pure reference, when its role is queried, then it inherits its referent role (quantities.md 5.6)"
bound = false

[[acceptance]]
text = "Given `head_dia <= 8.5mm`, `x >= 9.6mm` and `x in 4mm..6mm`, when constructed, then each is a Bound with min/max limits that is never emitted (quantities.md 5.7)"
bound = false
+++
