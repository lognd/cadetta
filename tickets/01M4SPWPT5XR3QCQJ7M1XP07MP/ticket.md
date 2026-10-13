+++
id = "01M4SPWPT5XR3QCQJ7M1XP07MP"
title = "Check statements with three-valued verdicts"
type = "story"
category = "todo"
priority = "medium"
points = 3
parent = "01M4SPWGB2729B40CQE81MHM2Z"
reporter = "lognd"
created = "2026-10-13T04:03:52Z"
updated = "2026-10-13T04:04:01Z"
persona = "design engineer"
capability = 'write `check name: lhs > rhs "msg"`'
outcome_text = "design rules are evaluated, not just commented"
labels = ["creates:crates/cadetta-sema/src/checks.rs"]
scope = ["crates/cadetta-sema/src/checks.rs"]

[[links]]
kind = "blocks"
target = "01M4SPWQH44JWFXZDNTGEBMMNX"

[[acceptance]]
text = "Given a check false for every value in tolerance, when evaluated, then verdict is violates"
bound = false

[[acceptance]]
text = "Given a check true for some values only, when evaluated, then verdict is undecided"
bound = false
+++
