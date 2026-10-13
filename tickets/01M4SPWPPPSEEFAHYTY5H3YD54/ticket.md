+++
id = "01M4SPWPPPSEEFAHYTY5H3YD54"
title = "Evaluation to exact values"
type = "story"
category = "todo"
priority = "medium"
points = 5
parent = "01M4SPWGB2729B40CQE81MHM2Z"
reporter = "lognd"
created = "2026-10-13T04:03:52Z"
updated = "2026-10-13T04:03:59Z"
persona = "design engineer"
capability = "evaluate every parameter to a quantity"
outcome_text = "derived values are exact and usable by lints and emitters"
labels = ["creates:crates/cadetta-sema/src/eval.rs"]
scope = ["crates/cadetta-sema/src/eval.rs"]

[[links]]
kind = "blocks"
target = "01M4SPWPT5XR3QCQJ7M1XP07MP"

[[links]]
kind = "blocks"
target = "01M4SPWQ0WTDXX64VS40QGESR5"

[[links]]
kind = "blocks"
target = "01M4SPWQ3VSX2CA3TZY4KQ2DZW"

[[acceptance]]
text = "Given `length = 2 * Wheel.hub_width + Frame.span` with 0.75in and 300mm, when evaluated, then the exact rational is 1.9m * ... equal to 338.1mm"
bound = false

[[acceptance]]
text = "Given `round_to(x, step)` on exact input, when evaluated, then the result is exact"
bound = false
+++
