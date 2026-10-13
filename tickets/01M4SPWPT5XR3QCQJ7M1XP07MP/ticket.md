+++
id = "01M4SPWPT5XR3QCQJ7M1XP07MP"
title = "Check statements with three-valued verdicts"
type = "story"
category = "in-progress"
priority = "medium"
points = 5
parent = "01M4SPWGB2729B40CQE81MHM2Z"
reporter = "lognd"
created = "2026-10-13T04:03:52Z"
updated = "2026-10-13T08:03:12Z"
persona = "design engineer"
capability = 'write `check name: lhs > rhs "msg"`'
outcome_text = "design rules are evaluated, not just commented"
labels = ["creates:crates/cadetta-sema/src/checks.rs"]
scope = ["crates/cadetta-sema/src/checks.rs", "crates/cadetta-sema/src/eval.rs"]

[[links]]
kind = "blocks"
target = "01M4SPWQH44JWFXZDNTGEBMMNX"

[[links]]
kind = "blocks"
target = "01M4STCDGQMN86JQJXKAQ2FX25"

[[acceptance]]
text = "Given a check false for every value in tolerance, when evaluated, then verdict is violates (C001) (quantities.md 5.8)"
bound = true

[[acceptance]]
text = "Given a check true for some values only, when evaluated, then verdict is undecided (C002) (quantities.md 5.8)"
bound = true

[[acceptance]]
text = "Given `check load_ok for w in wheel: ...`, when evaluated, then one verdict per instance is produced (language.md 6)"
bound = true

[[acceptance]]
text = "Given `x satisfies P` with P a bound, when evaluated, then x is tested against P and `P.min`/`P.max` read its limits (quantities.md 5.7)"
bound = true

[[acceptance]]
text = 'Given `require mass_budget: ... @customer:"REQ-12"`, when evaluated, then it is a check carrying a citation and tracked as an obligation (language.md 6)'
bound = true

[[acceptance]]
text = "Given a check name, when lowered, then it is the check canonical path (language.md 6)"
bound = true
+++
