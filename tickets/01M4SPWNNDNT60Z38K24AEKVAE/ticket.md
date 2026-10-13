+++
id = "01M4SPWNNDNT60Z38K24AEKVAE"
title = "Tolerances as affine forms over source variables"
type = "story"
category = "in-progress"
priority = "medium"
points = 8
parent = "01M4SPWFZP4041AN6YQE08X2AN"
reporter = "lognd"
created = "2026-10-13T04:03:51Z"
updated = "2026-10-13T06:20:20Z"
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

[[links]]
kind = "blocks"
target = "01M4SPWPXSG1YKHGF4KV5G43BT"

[[links]]
kind = "blocks"
target = "01M4ST7AB8627SAJ7MGB1GSRH6"

[[acceptance]]
text = "Given `10mm [+-0.05mm]`, `8mm [+0 -0.008mm]` and `12mm [-0.006mm -0.017mm]`, when constructed, then each is a lower<=upper deviation pair on the nominal (quantities.md 5.1)"
bound = false

[[acceptance]]
text = "Given lower > upper, or a deviation of a different dimension, when constructed, then an error value is returned (quantities.md 5.1)"
bound = false

[[acceptance]]
text = "Given `1in [+-0.1mm]`, when constructed, then deviation units may differ from the nominal unit (quantities.md 5.1)"
bound = false

[[acceptance]]
text = "Given `9.98mm..10.05mm`, when constructed, then the nominal is the midpoint (quantities.md 5.1)"
bound = false

[[acceptance]]
text = "Given `a + a` and `a - a` over one source variable, when evaluated, then repeated and cancelling terms are counted once, so `a - a` has zero tolerance (quantities.md 5.4)"
bound = false

[[acceptance]]
text = "Given a product of two toleranced values, when evaluated, then the result falls back to interval bounds and is marked conservative; Q005 no longer exists (quantities.md 5.4)"
bound = false

[[acceptance]]
text = "Given sqrt or trig of a toleranced value, when evaluated, then the result is conservative (quantities.md 5.4)"
bound = false
+++
