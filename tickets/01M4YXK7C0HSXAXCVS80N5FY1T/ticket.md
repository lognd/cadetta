+++
id = "01M4YXK7C0HSXAXCVS80N5FY1T"
title = "Sema: unindexed read of a counted part array in a solver in-expression (wheel.load_rating) is EVAL error"
type = "bug"
category = "in-progress"
priority = "medium"
points = 2
reporter = "lognd"
created = "2026-10-15T04:37:14Z"
updated = "2026-10-16T07:31:28Z"
scope = ["crates/cadetta-sema/**", "docs/spec/solvers.md", "tests/conformance/solvers/variants-*.expect.json", "tests/conformance/known-gaps.toml", "changelog.d/01M4YXK7C0HSXAXCVS80N5FY1T.fixed.md"]

[[acceptance]]
text = "Given a solver in-expression reading wheel.load_rating on part wheel[4], when evaluated, then it reads instance 0 (the primary or the selected alternate) with no EVAL error"
bound = true

[[acceptance]]
text = "Given a counted part array, when a member is read with an index on a non-array part, then it is still an error"
bound = true
+++

found while working ~NDGXJQ1. solvers/variants-flag-solves-alternates reads 'in load = wheel.load_rating' with 'part wheel[4]'; geom.rs through_parts rejects it ('part array; read one instance') so solve --variants cannot run. solvers.md 4.1 names wheel.load_rating as the example read. Needs a spec/sema decision: allow unindexed reads of identical instances in in-expressions (or change the fixture to wheel[0]).
