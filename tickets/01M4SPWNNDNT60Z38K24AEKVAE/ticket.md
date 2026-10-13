+++
id = "01M4SPWNNDNT60Z38K24AEKVAE"
title = "Tolerances and worst-case interval arithmetic"
type = "story"
category = "todo"
priority = "medium"
points = 5
parent = "01M4SPWFZP4041AN6YQE08X2AN"
reporter = "lognd"
created = "2026-10-13T04:03:51Z"
updated = "2026-10-13T04:03:58Z"
persona = "design engineer"
capability = "attach symmetric, asymmetric, limit and fit tolerances to quantities and add them"
outcome_text = "stacked deviations are computed exactly"
labels = ["creates:crates/cadetta-units/src/tolerance.rs"]
scope = ["crates/cadetta-units/src/tolerance.rs"]

[[links]]
kind = "blocks"
target = "01M4SPWNR85DB2J5CGXZK9NP4M"

[[links]]
kind = "blocks"
target = "01M4SPWPPPSEEFAHYTY5H3YD54"

[[acceptance]]
text = "Given `10mm +- 0.05mm` plus `5mm +0.1mm -0.02mm`, when added, then the deviations add (worst case)"
bound = false

[[acceptance]]
text = "Given two toleranced quantities, when multiplied, then Q005 is signalled"
bound = false

[[acceptance]]
text = "Given lower > upper, when constructed, then it is rejected"
bound = false
+++
