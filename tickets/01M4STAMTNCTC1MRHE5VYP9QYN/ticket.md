+++
id = "01M4STAMTNCTC1MRHE5VYP9QYN"
title = "External solvers (cadetta-solvers)"
type = "epic"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T05:03:55Z"
updated = "2026-10-13T05:03:55Z"
idempotency_key = "replan13:External solvers (cadetta-solvers)"
labels = ["creates:crates/cadetta-solvers/**"]
scope = ["crates/cadetta-solvers/**"]
+++

Outcome: computed values from external programs (FEA, CFD, scripts) enter the design as cited, digest-pinned results, per docs/spec/solvers.md.

Success metric: a solver result changes only when its input digest changes, and variants get their own results.
