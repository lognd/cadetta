+++
id = "01M4SPWPPPSEEFAHYTY5H3YD54"
title = "Evaluation to exact values and analysis lets"
type = "story"
category = "in-progress"
priority = "medium"
points = 8
parent = "01M4SPWGB2729B40CQE81MHM2Z"
reporter = "lognd"
created = "2026-10-13T04:03:52Z"
updated = "2026-10-13T07:28:13Z"
persona = "design engineer"
capability = "evaluate every parameter to a quantity"
outcome_text = "derived values are exact and usable by lints and emitters"
labels = ["creates:crates/cadetta-sema/src/eval.rs"]
scope = ["crates/cadetta-sema/src/eval.rs", "crates/cadetta-sema/src/lib.rs", "docs/spec/quantities.md"]

[[links]]
kind = "blocks"
target = "01M4SPWPT5XR3QCQJ7M1XP07MP"

[[links]]
kind = "blocks"
target = "01M4SPWQ0WTDXX64VS40QGESR5"

[[links]]
kind = "blocks"
target = "01M4SPWQ3VSX2CA3TZY4KQ2DZW"

[[links]]
kind = "blocks"
target = "01M4ST7AB8627SAJ7MGB1GSRH6"

[[links]]
kind = "blocks"
target = "01M4ST7ADMK0S7SRW6ETCS8ETC"

[[links]]
kind = "blocks"
target = "01M4ST7AFZGCN52434BQJRT5MM"

[[links]]
kind = "blocks"
target = "01M4ST7AKENKKS96TTXY13YMZQ"

[[links]]
kind = "blocks"
target = "01M4ST7APCCWC8KASSNHK0FW70"

[[links]]
kind = "blocks"
target = "01M4ST7ASWYQV13AAHHCXVR570"

[[links]]
kind = "blocks"
target = "01M4ST7AX1C1SYB67DVDW1EB06"

[[acceptance]]
text = "Given `round_to(x, step)` on exact input, when evaluated, then the result is exact"
bound = true

[[acceptance]]
text = "Given `length = 2 * Wheel.hub_width + Frame.span` with 0.75in and 300mm, when evaluated, then the exact value equals 338.1mm"
bound = true

[[acceptance]]
text = "Given a member `let`, when evaluated, then it keeps its full analysis form, is readable as a part argument and from other components, and is flagged analysis (language.md 3.3, quantities.md 5.3)"
bound = false

[[acceptance]]
text = "Given a parameter assigned from arithmetic, when evaluated, then it stores the nominal and only its own written tolerance (quantities.md 5.3)"
bound = false

[[acceptance]]
text = "Given a pure reference parameter, when evaluated, then it aliases the referent source variable (quantities.md 5.3)"
bound = false

[[acceptance]]
text = "Given the example `cb = head_dia.max + 1mm`, when evaluated, then an untoleranced 9.5mm is stored (quantities.md 5.3)"
bound = false
+++
