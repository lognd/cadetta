+++
id = "01M4STAN5C20ACNDRARX5MRDDP"
title = "Result files keyed by input digest"
type = "story"
category = "todo"
priority = "medium"
points = 5
parent = "01M4STAMTNCTC1MRHE5VYP9QYN"
reporter = "lognd"
created = "2026-10-13T05:03:55Z"
updated = "2026-10-13T05:04:22Z"
persona = "design engineer"
capability = "keep solver results in the repo keyed by their inputs"
idempotency_key = "replan13:Result files keyed by input digest"
labels = ["creates:crates/cadetta-solvers/src/results.rs"]
scope = ["crates/cadetta-solvers/src/results.rs"]

[[acceptance]]
text = "Given a run, when stored, then `<script>.out.json` holds results keyed by input digest (solvers.md 4)"
bound = false

[[acceptance]]
text = "Given unchanged inputs, when evaluated, then the cached result is used and the solver is not run (solvers.md 4)"
bound = false

[[acceptance]]
text = "Given changed inputs, when evaluated, then the output is stale but still evaluates until re-solved (solvers.md 4)"
bound = false

[[acceptance]]
text = "Given result digests, when recorded, then they are written to cadetta.lock (obligations.md 3)"
bound = false
+++
