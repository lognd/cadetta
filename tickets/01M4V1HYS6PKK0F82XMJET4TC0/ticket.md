+++
id = "01M4V1HYS6PKK0F82XMJET4TC0"
title = "Solver and sync obligations not produced by check (S003, O004)"
type = "bug"
category = "in-progress"
priority = "medium"
points = 3
reporter = "lognd"
created = "2026-10-13T16:29:29Z"
updated = "2026-10-15T19:38:04Z"
scope = ["crates/cadetta-cli/src/**"]

[[links]]
kind = "blocked-by"
target = "01M4YRH3G6F8EGQ5X5CK4JKH27"
+++

solvers.md 4 / obligations.md 2.3: a solver member declaration is E002 unresolved in check, no S003 staleness from .out.json and no O004 sync obligation. found while working ~P3E2FR7
