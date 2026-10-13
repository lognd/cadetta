+++
id = "01M4SWK96MJF28HVDDJNPCBMPP"
title = "IR entities Datum, Point, Placement and shared Op"
type = "story"
category = "in-progress"
priority = "medium"
points = 5
parent = "01M4SPWGV64X8H1TN1YBR6K5CH"
reporter = "lognd"
created = "2026-10-13T05:43:35Z"
updated = "2026-10-13T08:55:49Z"
persona = "design engineer"
capability = "see positions in the IR"
idempotency_key = "replan14:ir-geo"
labels = ["creates:crates/cadetta-ir/src/geometry.rs"]
scope = ["crates/cadetta-ir/src/geometry.rs", "crates/cadetta-ir/src/lib.rs", "crates/cadetta-ir/src/schema.rs", "docs/schemas/cadetta.ir.1.json", "docs/spec/ir.md"]

[[links]]
kind = "blocked-by"
target = "01M4SPWQ3VSX2CA3TZY4KQ2DZW"

[[links]]
kind = "blocked-by"
target = "01M4SWFX374CNSPDDD289Z9BCS"

[[links]]
kind = "blocked-by"
target = "01M4SWFXC2K6D3S93F3ER0GZZA"

[[acceptance]]
text = "Given a component with datums and points, when lowered, then Datum {path, parent, ops, variants?, digest, closure} and Point {path, datum, coords} are emitted (ir.md 3)"
bound = true

[[acceptance]]
text = "Given placements, when lowered, then each is an edge {part, datum, on, ops} of the part tree with variants like members (ir.md 3; geometry.md 6)"
bound = true

[[acceptance]]
text = "Given Op = offset | rotate | flip | at, when serialized, then Datum and Placement share one form and the composed transform is derived, not stored (ir.md 3)"
bound = true
+++
