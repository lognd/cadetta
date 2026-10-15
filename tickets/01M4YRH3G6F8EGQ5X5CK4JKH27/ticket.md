+++
id = "01M4YRH3G6F8EGQ5X5CK4JKH27"
title = "Sema: resolve solver member outputs (solver.out) so check evaluates them; S003/S006 staleness needs it"
type = "bug"
category = "todo"
priority = "medium"
points = 5
reporter = "lognd"
created = "2026-10-15T03:08:41Z"
updated = "2026-10-15T18:10:48Z"
scope = ["crates/cadetta-sema/src/**", "crates/cadetta-cli/src/**"]
+++

found while working ~JET4TC0. resolve.rs/eval.rs have no notion of a solver member (MemberKind::Solver): wall_sizing.min_wall is E002 in check. Needed: resolver binds solver names as component members, evaluator yields the stored .out.json output (cadetta-solve outputs::use_output) and attaches S003 facts; cli pipeline then emits S003/S006. Unblocks the solvers/* and obligations/s003-solver-stale known-gaps.
