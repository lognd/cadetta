+++
id = "01M4YXK7C0HSXAXCVS80N5FY1T"
title = "Sema: unindexed read of a counted part array in a solver in-expression (wheel.load_rating) is EVAL error"
type = "bug"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-15T04:37:14Z"
updated = "2026-10-15T04:37:14Z"
scope = ["crates/cadetta-sema/**"]
+++

found while working ~NDGXJQ1. solvers/variants-flag-solves-alternates reads 'in load = wheel.load_rating' with 'part wheel[4]'; geom.rs through_parts rejects it ('part array; read one instance') so solve --variants cannot run. solvers.md 4.1 names wheel.load_rating as the example read. Needs a spec/sema decision: allow unindexed reads of identical instances in in-expressions (or change the fixture to wheel[0]).
