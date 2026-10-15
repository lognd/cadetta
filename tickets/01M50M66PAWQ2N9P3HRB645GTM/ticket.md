+++
id = "01M50M66PAWQ2N9P3HRB645GTM"
title = "S006 emission and recorded variant outputs in check and evaluators"
type = "task"
category = "in-progress"
priority = "medium"
points = 3
reporter = "lognd"
created = "2026-10-15T20:31:19Z"
updated = "2026-10-15T21:50:45Z"
scope = ["crates/cadetta-cli/src/**", "docs/spec/solvers.md", "tests/conformance/known-gaps.toml"]

[[acceptance]]
text = "Given a variant whose solver inputs change and has no recorded result, when checked, then S006 is raised and its dependent checks are undecided (solvers.md 4.1)"
bound = true

[[acceptance]]
text = "Given a variant whose solver inputs equal the primary or that has its own recorded result, when checked, then it reads that result and S006 does not fire"
bound = false
+++

Split from ~JET4TC0 (S003 landed). Remaining: S006 emission, variant evaluator in pipeline.rs and evaluators in cmd/solve.rs get recorded variant outputs; fixtures still in known-gaps.toml.
