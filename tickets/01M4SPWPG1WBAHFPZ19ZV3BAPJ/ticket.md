+++
id = "01M4SPWPG1WBAHFPZ19ZV3BAPJ"
title = "Dependency graph, cycles and renames"
type = "story"
category = "done"
outcome = "done"
priority = "medium"
points = 3
parent = "01M4SPWGB2729B40CQE81MHM2Z"
reporter = "lognd"
created = "2026-10-13T04:03:52Z"
updated = "2026-10-13T07:11:00Z"
persona = "design engineer"
capability = "get cycle detection and `was` rename resolution"
outcome_text = "single-source-of-truth errors are caught"
labels = ["creates:crates/cadetta-sema/src/deps.rs"]
scope = ["crates/cadetta-sema/src/deps.rs", "crates/cadetta-sema/src/lib.rs", "docs/spec/language.md"]

[[acceptance]]
text = "Given member a depends on b and b on a, when analysed, then D003 reference-cycle is reported (language.md 4)"
bound = true

[[acceptance]]
text = "Given a model argument that comes from a joint reading another member of the same model, when analysed, then no cycle is reported because D003 is per member, not per component (language.md 4)"
bound = true

[[acceptance]]
text = "Given a part count expression that reads a member depending on the part, when analysed, then D003 is reported (language.md 3.5)"
bound = true

[[acceptance]]
text = "Given a reference to a name listed in `was`, when resolved, then it resolves and N004 is emitted (language.md 3.3)"
bound = true

[[acceptance]]
text = "Given the dependency graph, when built, then it is known without evaluating the design (language.md intro)"
bound = true
+++
