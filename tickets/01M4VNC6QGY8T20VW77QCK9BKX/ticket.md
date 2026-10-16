+++
id = "01M4VNC6QGY8T20VW77QCK9BKX"
title = "IR Solver carries citation: CiteRef when solvers are lowered"
type = "task"
category = "in-progress"
priority = "low"
points = 1
parent = "01M4VB1MJC8NTMD6AAZR5Y5J08"
reporter = "lognd"
created = "2026-10-13T22:15:52Z"
updated = "2026-10-16T03:11:06Z"
scope = ["crates/cadetta-ir/src/**", "crates/cadetta-sema/src/lower.rs", "crates/cadetta-sema/src/lower_solver.rs", "crates/cadetta-cli/src/cmd/impact.rs", "crates/cadetta-cli/src/universe.rs", "crates/cadetta-lint/src/rules/d.rs", "docs/spec/ir.md", "docs/schemas/cadetta.ir.1.json"]

[[acceptance]]
text = "Given a solver with @cite, when the IR Solver is built, then Solver.citation is its CiteRef and not an input to the solver closure (ir.md 3; solvers.md 1)"
bound = true
+++

Left by ~253FATR: SolverDecl carries the citation syntactically, but the IR Solver type has no citation field and solvers are not lowered yet.
