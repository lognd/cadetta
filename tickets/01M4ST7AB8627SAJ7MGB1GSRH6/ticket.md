+++
id = "01M4ST7AB8627SAJ7MGB1GSRH6"
title = "Affine tolerance evaluation with source variables"
type = "story"
category = "in-progress"
priority = "medium"
points = 8
parent = "01M4SPWGB2729B40CQE81MHM2Z"
reporter = "lognd"
created = "2026-10-13T05:02:06Z"
updated = "2026-10-13T08:34:38Z"
persona = "design engineer"
capability = "see worst-case checks over the tolerance of every source variable they reach"
idempotency_key = "replan13:Affine tolerance evaluation with source variables"
labels = ["creates:crates/cadetta-sema/src/affine.rs"]
scope = ["crates/cadetta-sema/src/affine.rs"]

[[acceptance]]
text = "Given each toleranced parameter of each part instance, when evaluated, then it is one independent source variable (quantities.md 5.4)"
bound = true

[[acceptance]]
text = "Given two instances of one part, when both are toleranced, then they vary independently (quantities.md 5.4, OQ-9)"
bound = true

[[acceptance]]
text = "Given a pure reference parameter, when evaluated, then it shares its referent source variable, so housing.groove_width and ring.groove_width vary together (quantities.md 5.3)"
bound = true

[[acceptance]]
text = "Given a parameter assigned from toleranced arithmetic, when evaluated, then it becomes its own source variable carrying only its written tolerance (quantities.md 5.3)"
bound = true

[[acceptance]]
text = "Given a `let` stack-up, when a check reads it, then it sees every tolerance upstream and repeated terms count once (quantities.md 5.3, 5.4)"
bound = true

[[acceptance]]
text = "Given a check reading an arithmetic parameter with no tolerance of its own whose inputs are toleranced, when analysed, then the C004 fact is recorded (quantities.md 5.3)"
bound = true

[[acceptance]]
text = "Given a nonlinear operation, when evaluated, then the result falls back to interval bounds and is marked conservative (quantities.md 5.4)"
bound = true
+++
