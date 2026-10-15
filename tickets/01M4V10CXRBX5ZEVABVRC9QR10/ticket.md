+++
id = "01M4V10CXRBX5ZEVABVRC9QR10"
title = "IR JSON Schema lacks Project interfaces/tables/requires/waivers and Component datums/points/placements"
type = "bug"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T16:19:53Z"
updated = "2026-10-15T01:44:58Z"
scope = ["docs/schemas/**", "crates/cadetta-ir/src/schema.rs", "docs/spec/ir.md", "changelog.d/01M4V10CXRBX5ZEVABVRC9QR10.*"]

[[acceptance]]
text = "Project schema accepts optional interfaces, tables and waivers"
bound = true

[[acceptance]]
text = "Committed schema equals the generated schema"
bound = false
+++

found while working ~99NMCBG: ir.md 3 lists these on Project and Component but cadetta.ir.1.json omits them (Interface, Table, Waiver, Process exist only under defs). Fixtures datum-variants-chain, op-* and path-star-bindings-only use datums/placements and fail the committed schema until it is extended.
