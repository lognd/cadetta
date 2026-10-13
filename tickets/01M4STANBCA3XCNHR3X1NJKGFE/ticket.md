+++
id = "01M4STANBCA3XCNHR3X1NJKGFE"
title = "solve --variants and per-variant results"
type = "story"
category = "in-progress"
priority = "medium"
points = 5
parent = "01M4STAMTNCTC1MRHE5VYP9QYN"
reporter = "lognd"
created = "2026-10-13T05:03:55Z"
updated = "2026-10-13T13:53:50Z"
persona = "design engineer"
capability = "solve each approved alternate with its own inputs"
idempotency_key = "replan13:solve --variants and per-variant results"
scope = ["crates/cadetta-solve/src/variants.rs", "crates/cadetta-solve/src/lib.rs", "crates/cadetta-cli/src/cmd/solve.rs", "crates/cadetta-cli/src/cmd/mod.rs"]

[[acceptance]]
text = "Given an alternate that changes a solver input, when `cadetta solve --variants` runs, then one result per distinct input digest is stored (solvers.md 4.1)"
bound = true

[[acceptance]]
text = "Given variant evaluation without a result, when evaluated, then the value is unknown, not the primary result (solvers.md 4.1)"
bound = true
+++
