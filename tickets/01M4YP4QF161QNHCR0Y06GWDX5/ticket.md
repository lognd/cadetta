+++
id = "01M4YP4QF161QNHCR0Y06GWDX5"
title = "IR decoder: tables, checks, bindings, interfaces, waivers, datums/points/placements; real line/col for lint FILE"
type = "task"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-15T02:26:59Z"
updated = "2026-10-15T02:26:59Z"
scope = ["crates/cadetta-ir/src/decode.rs", "crates/cadetta-cli/src/cmd/ir_lint.rs"]
+++

found while working ~71ZWJXV: decoder skips these entities, so D002 sees no table cells and lint FILE reports line 1.
