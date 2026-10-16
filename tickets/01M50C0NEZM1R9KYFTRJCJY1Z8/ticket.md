+++
id = "01M50C0NEZM1R9KYFTRJCJY1Z8"
title = "Syntax/sema/IR: [position d] on Point2, E006 rules, Point.position and .u/.v schema"
type = "task"
category = "in-progress"
priority = "medium"
points = 3
reporter = "lognd"
created = "2026-10-15T18:08:29Z"
updated = "2026-10-16T05:04:00Z"
idempotency_key = "v2k95h5-slice-frontend"
scope = ["crates/cadetta-syntax/src/**", "crates/cadetta-ir/src/**", "crates/cadetta-cli/src/**", "crates/cadetta-solve/src/**", "crates/cadetta-geometry/src/placement.rs", "docs/spec/*.md"]

[[links]]
kind = "blocked-by"
target = "01M50C0NA43G0H8N5YSRDMWEB8"

[[acceptance]]
text = "A Point2 with [position d] lowers to paired .u/.v terms and the E006 cases fire."
bound = false
+++

Parse [position d], lower one disc per point, E006 for Point3 / coordinate tolerance on same value / toleranced generator member, Point.position in IR model and docs/schemas/cadetta.ir.1.json, holes[position] stays an index. Blocked by the units slice. Split from ~V2K95H5 (too large for one slice).
