+++
id = "01M4ZJAYRJMG47610S0R0ANADA"
title = "Lower solvers, tables and waivers into the IR in cadetta-sema"
type = "task"
category = "done"
outcome = "done"
priority = "medium"
points = 5
reporter = "lognd"
created = "2026-10-15T10:39:43Z"
updated = "2026-10-16T03:02:38Z"
scope = ["crates/cadetta-sema/**", "crates/cadetta-cli/src/pipeline.rs", "docs/spec/ir.md"]

[[acceptance]]
text = "Solvers and waivers declared in a package lower into ir.solvers and ir.waivers (solver deps, inputs, outputs, result state); tables are filed as a follow-up"
bound = true
+++

found while working TNGHA7Q: ir.solvers is empty so solver: obligations are unreachable end to end
