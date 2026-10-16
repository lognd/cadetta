+++
id = "01M5187T6Y9B3SMBS9DMPG9VFX"
title = "Lower tables into the IR in cadetta-sema"
type = "task"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-16T02:21:43Z"
updated = "2026-10-16T02:21:43Z"
scope = ["crates/cadetta-sema/**"]

[[acceptance]]
text = "Tables declared in a package lower into ir.tables with their cell values"
bound = false
+++

found while working ~R0ANADA: ir.tables is empty; the evaluator keeps table cell values private (table_vals), so lowering needs them exposed from Evaluation.
