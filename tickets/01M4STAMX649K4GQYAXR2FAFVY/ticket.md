+++
id = "01M4STAMX649K4GQYAXR2FAFVY"
title = "Solver declaration sema and IR"
type = "story"
category = "in-progress"
priority = "medium"
points = 5
parent = "01M4STAMTNCTC1MRHE5VYP9QYN"
reporter = "lognd"
created = "2026-10-13T05:03:55Z"
updated = "2026-10-13T06:57:56Z"
persona = "design engineer"
capability = "declare an external solver with typed inputs and outputs"
idempotency_key = "replan13:Solver declaration sema and IR"
labels = ["creates:crates/cadetta-solve/src/decl.rs"]
scope = ["crates/cadetta-solve/Cargo.toml", "crates/cadetta-solve/src/lib.rs", "docs/spec/solvers.md", "docs/spec/lints.md", "Cargo.lock", "crates/cadetta-solve/src/decl.rs"]

[[links]]
kind = "blocks"
target = "01M4STAMZCNT7MPR1PGZ5GVZZ5"

[[links]]
kind = "blocks"
target = "01M4STANDBYZAR6T9BXPKHYYYV"

[[acceptance]]
text = "Given a `solver` declaration with typed inputs, outputs and a script path, when lowered, then the IR carries it with its closure (solvers.md 2, ir.md 2)"
bound = true

[[acceptance]]
text = "Given an input expression that is not a literal or SI-convertible value, when checked, then an error is reported (solvers.md 1)"
bound = true

[[acceptance]]
text = "Given outputs, when lowered, then each is a bound or driving output (solvers.md 1)"
bound = true
+++
