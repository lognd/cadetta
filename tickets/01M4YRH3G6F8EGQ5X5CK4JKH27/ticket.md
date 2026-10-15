+++
id = "01M4YRH3G6F8EGQ5X5CK4JKH27"
title = "Sema: resolve solver member outputs (solver.out) so check evaluates them; S003/S006 staleness needs it"
type = "bug"
category = "in-progress"
priority = "medium"
points = 5
reporter = "lognd"
created = "2026-10-15T03:08:41Z"
updated = "2026-10-15T18:39:54Z"
scope = ["crates/cadetta-sema/src/**", "crates/cadetta-cli/src/**", "tests/conformance/known-gaps.toml"]

[[acceptance]]
text = "Sema resolves solver member outputs (solver.out) so check and ir evaluate them without E002"
bound = false

[[acceptance]]
text = "A recorded solver output literal is evaluated exactly and appears in the IR as an analysis member"
bound = false
+++

found while working ~JET4TC0. resolve.rs/eval.rs have no notion of a solver member (MemberKind::Solver): wall_sizing.min_wall is E002 in check. Needed: resolver binds solver names as component members, evaluator yields the stored .out.json output (cadetta-solve outputs::use_output) and attaches S003 facts; cli pipeline then emits S003/S006. Unblocks the solvers/* and obligations/s003-solver-stale known-gaps.
