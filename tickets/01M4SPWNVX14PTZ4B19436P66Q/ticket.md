+++
id = "01M4SPWNVX14PTZ4B19436P66Q"
title = "Inexact values and rounding (sqrt, trig, round_to)"
type = "story"
category = "in-progress"
priority = "medium"
points = 5
parent = "01M4SPWFZP4041AN6YQE08X2AN"
reporter = "lognd"
created = "2026-10-13T04:03:51Z"
updated = "2026-10-13T06:22:07Z"
persona = "design engineer"
capability = "use sqrt, sin, cos, tan, atan2 and non-integer powers"
outcome_text = "inexactness is flagged and propagates visibly"
labels = ["creates:crates/cadetta-units/src/inexact.rs"]
scope = ["crates/cadetta-units/src/inexact.rs"]

[[acceptance]]
text = "Given sqrt(2mm^2), when evaluated, then the value is marked inexact and carried as a binary64 naming its root operation (quantities.md 1)"
bound = false

[[acceptance]]
text = "Given an inexact operand, when added to an exact one, then the result is inexact (quantities.md 1)"
bound = false

[[acceptance]]
text = "Given sin/cos/tan of exact multiples of 90deg and the matching multiples of 30deg and 45deg, when evaluated, then the result is exact; atan2(3in, 3in) == 45deg holds (quantities.md 4.3)"
bound = false

[[acceptance]]
text = "Given `round_to`, `ceil_to` and `floor_to` of an exact value, when evaluated, then the result is an exact rational multiple of step, ties away from zero, deviations kept (quantities.md 5.5)"
bound = false

[[acceptance]]
text = "Given an inexact value within 1e-9 relative of a tie or step boundary, when rounded, then Q010 ambiguous-rounding is returned (quantities.md 5.5)"
bound = false

[[acceptance]]
text = "Given a step of a different dimension, when rounding, then an error value is returned (quantities.md 5.5)"
bound = false
+++
