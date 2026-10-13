+++
id = "01M4STANDBYZAR6T9BXPKHYYYV"
title = "Bound versus driving outputs"
type = "story"
category = "in-progress"
priority = "medium"
points = 5
parent = "01M4STAMTNCTC1MRHE5VYP9QYN"
reporter = "lognd"
created = "2026-10-13T05:03:55Z"
updated = "2026-10-13T07:34:54Z"
persona = "design engineer"
capability = "use solver outputs as checked values or as drivers"
idempotency_key = "replan13:Bound versus driving outputs"
labels = ["creates:crates/cadetta-solvers/src/outputs.rs"]
scope = ["crates/cadetta-solvers/src/outputs.rs", "crates/cadetta-solve/src/outputs.rs", "crates/cadetta-solve/src/lib.rs", "docs/spec/solvers.md"]

[[acceptance]]
text = "Given a bound output, when evaluated, then it is checked against the design and never drives it (solvers.md 2)"
bound = true

[[acceptance]]
text = "Given a driving output, when evaluated, then downstream values derive from it and cite the solver (solvers.md 2)"
bound = true

[[acceptance]]
text = "Given a stale output, when evaluated, then it still evaluates with the S003 fact (solvers.md 4.1, lints.md S)"
bound = false
+++
