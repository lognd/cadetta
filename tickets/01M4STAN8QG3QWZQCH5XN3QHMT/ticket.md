+++
id = "01M4STAN8QG3QWZQCH5XN3QHMT"
title = "cadetta solve"
type = "story"
category = "in-progress"
priority = "medium"
points = 3
parent = "01M4STAMTNCTC1MRHE5VYP9QYN"
reporter = "lognd"
created = "2026-10-13T05:03:55Z"
updated = "2026-10-13T07:56:26Z"
persona = "design engineer"
capability = "run stale solvers on demand"
idempotency_key = "replan13:cadetta solve"
labels = ["creates:crates/cadetta-cli/src/cmd/solve.rs"]
scope = ["crates/cadetta-cli/src/cmd/solve.rs", "crates/cadetta-cli/src/lib.rs", "crates/cadetta-cli/src/cmd/mod.rs", "crates/cadetta-sema/src/eval.rs", "docs/spec/solvers.md"]

[[acceptance]]
text = "Given stale or missing results, when `cadetta solve [NAME]` runs, then the solver runs and results are stored (solvers.md 4)"
bound = false

[[acceptance]]
text = "Given nothing stale, when run, then nothing runs and exit is 0 (solvers.md 4)"
bound = false
+++
