+++
id = "01M4SPWPG1WBAHFPZ19ZV3BAPJ"
title = "Dependency graph, cycles and renames"
type = "story"
category = "todo"
priority = "medium"
points = 3
parent = "01M4SPWGB2729B40CQE81MHM2Z"
reporter = "lognd"
created = "2026-10-13T04:03:52Z"
updated = "2026-10-13T04:03:52Z"
persona = "design engineer"
capability = "get cycle detection and `was` rename resolution"
outcome_text = "single-source-of-truth errors are caught"
labels = ["creates:crates/cadetta-sema/src/deps.rs"]
scope = ["crates/cadetta-sema/src/deps.rs"]

[[acceptance]]
text = "Given a depends on b and b on a, when analysed, then D003 reference-cycle is reported"
bound = false

[[acceptance]]
text = "Given a reference to a name listed in `was`, when resolved, then it resolves and N004 is emitted"
bound = false
+++
