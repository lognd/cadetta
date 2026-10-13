+++
id = "01M4ST7APCCWC8KASSNHK0FW70"
title = "Processes and drawn versus designed values"
type = "story"
category = "in-progress"
priority = "medium"
points = 5
parent = "01M4SPWGB2729B40CQE81MHM2Z"
reporter = "lognd"
created = "2026-10-13T05:02:06Z"
updated = "2026-10-13T08:21:17Z"
persona = "design engineer"
capability = "compensate printed holes once, in a process, without hand-adjusting dimensions"
idempotency_key = "replan13:Processes and drawn versus designed values"
labels = ["creates:crates/cadetta-sema/src/process.rs"]
scope = ["crates/cadetta-sema/src/process.rs", "crates/cadetta-sema/src/lib.rs", "docs/spec/language.md"]

[[acceptance]]
text = "Given `process Fdm` with `rule hole(d)` and a member tagged `#[hole]`, when evaluated, then designed holds the part as made and drawn holds the rule applied to nominals (language.md 3.8)"
bound = true

[[acceptance]]
text = "Given fits, checks and reads, when evaluated under a process, then they see the designed value and `x.drawn` reads the drawn value (language.md 3.8)"
bound = true

[[acceptance]]
text = "Given a toleranced process parameter `hole_comp [+-0.05mm]`, when evaluated, then it is one source variable added to the designed value of every parameter its rules touch (language.md 3.8)"
bound = true

[[acceptance]]
text = "Given a parameter without a role under `#![process(Fdm)]`, when evaluated, then it is not compensated; roles have no `#!` default (language.md 3.8)"
bound = true

[[acceptance]]
text = "Given a feature-like parameter name without a role, when analysed, then the P001 fact is recorded (language.md 3.8)"
bound = true

[[acceptance]]
text = "Given emit, when a compensated member is written, then the drawn value is written (language.md 3.8)"
bound = true
+++
