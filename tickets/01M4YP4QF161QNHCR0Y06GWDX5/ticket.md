+++
id = "01M4YP4QF161QNHCR0Y06GWDX5"
title = "IR decoder: tables, checks, bindings, interfaces, waivers, datums/points/placements; real line/col for lint FILE"
type = "task"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-15T02:26:59Z"
updated = "2026-10-15T02:32:30Z"
scope = ["crates/cadetta-ir/src/decode.rs", "crates/cadetta-cli/src/cmd/ir_lint.rs", "crates/cadetta-cli/tests/ir_lint_rules.rs", "docs/spec/config-cli.md", "changelog.d/01M4YP4QF161QNHCR0Y06GWDX5.added.md"]

[[acceptance]]
text = "The decoder reads dependencies, checks, bindings, interfaces, tables, waivers and component datums, points and placements and round-trips them"
bound = true

[[acceptance]]
text = "lint FILE reports D002 for a literal equal to a table cell and stays quiet otherwise"
bound = false

[[acceptance]]
text = "lint FILE fires F004 and N001 on decoded fits and member names"
bound = false

[[acceptance]]
text = "lint FILE reports real line and column from the source file beside the IR document"
bound = false
+++

found while working ~71ZWJXV: decoder skips these entities, so D002 sees no table cells and lint FILE reports line 1.
