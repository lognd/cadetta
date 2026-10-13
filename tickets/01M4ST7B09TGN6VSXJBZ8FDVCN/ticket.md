+++
id = "01M4ST7B09TGN6VSXJBZ8FDVCN"
title = "Observed IR flavour and expression capture"
type = "story"
category = "todo"
priority = "medium"
points = 3
parent = "01M4SPWGV64X8H1TN1YBR6K5CH"
reporter = "lognd"
created = "2026-10-13T05:02:06Z"
updated = "2026-10-13T05:02:06Z"
persona = "design engineer"
capability = "lint existing CAD files before I write any source"
idempotency_key = "replan13:Observed IR flavour and expression capture"
labels = ["creates:crates/cadetta-ir/src/observed.rs"]
scope = ["crates/cadetta-ir/src/observed.rs"]

[[acceptance]]
text = "Given an adapter read result, when built, then parameters carry native name, locator, unit_assumed and exact:false with a reason where not faithful (ir.md 1, adapters.md 3)"
bound = false

[[acceptance]]
text = 'Given a native expression such as `"A" = "B" * 2`, when built, then observed IR holds the expression (ir.md 1)'
bound = false

[[acceptance]]
text = "Given value lints Q, N, D001 and M001, when run on observed IR, then they work without design-only fields (ir.md 1)"
bound = false
+++
