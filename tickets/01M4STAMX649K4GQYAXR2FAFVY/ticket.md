+++
id = "01M4STAMX649K4GQYAXR2FAFVY"
title = "Solver declaration sema and IR"
type = "story"
category = "todo"
priority = "medium"
points = 5
parent = "01M4STAMTNCTC1MRHE5VYP9QYN"
reporter = "lognd"
created = "2026-10-13T05:03:55Z"
updated = "2026-10-13T05:04:20Z"
persona = "design engineer"
capability = "declare an external solver with typed inputs and outputs"
idempotency_key = "replan13:Solver declaration sema and IR"
labels = ["creates:crates/cadetta-solvers/src/decl.rs"]
scope = ["crates/cadetta-solvers/src/decl.rs"]

[[acceptance]]
text = "Given a `solver` declaration with typed inputs, outputs and a script path, when lowered, then the IR carries it with its closure (solvers.md 2, ir.md 2)"
bound = false

[[acceptance]]
text = "Given an input expression that is not a literal or SI-convertible value, when checked, then an error is reported (solvers.md 1)"
bound = false

[[acceptance]]
text = "Given outputs, when lowered, then each is a bound or driving output (solvers.md 1)"
bound = false
+++
