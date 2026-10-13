+++
id = "01M4SPWNVX14PTZ4B19436P66Q"
title = "Inexact values (sqrt, trig, non-integer powers)"
type = "story"
category = "todo"
priority = "medium"
points = 3
parent = "01M4SPWFZP4041AN6YQE08X2AN"
reporter = "lognd"
created = "2026-10-13T04:03:51Z"
updated = "2026-10-13T04:03:51Z"
persona = "design engineer"
capability = "use sqrt, sin, cos, tan, atan2 and non-integer powers"
outcome_text = "inexactness is flagged and propagates visibly"
labels = ["creates:crates/cadetta-units/src/inexact.rs"]
scope = ["crates/cadetta-units/src/inexact.rs"]

[[acceptance]]
text = "Given sqrt(2mm^2), when evaluated, then the value is marked inexact with a binary64 approximation"
bound = false

[[acceptance]]
text = "Given an inexact operand, when added to an exact one, then the result is inexact"
bound = false
+++
