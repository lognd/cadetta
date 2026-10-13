+++
id = "01M4SWFXADMC8C5Y451JRVBYZM"
title = "Pattern generators rect_pattern and circle_pattern with bindable members"
type = "story"
category = "in-progress"
priority = "medium"
points = 5
parent = "01M4SWEQMQ8DQMN8B8GWVR4GDH"
reporter = "lognd"
created = "2026-10-13T05:41:44Z"
updated = "2026-10-13T13:38:36Z"
persona = "design engineer"
capability = "tolerance a hole pattern by its pitch"
idempotency_key = "replan14:geo-pattern"
labels = ["creates:crates/cadetta-geometry/src/pattern.rs"]
scope = ["crates/cadetta-geometry/src/pattern.rs", "crates/cadetta-geometry/src/lib.rs", "crates/cadetta-geometry/src/transform.rs", "docs/spec/geometry.md", "crates/cadetta-geometry/src/op_members.rs"]

[[links]]
kind = "blocked-by"
target = "01M4SWFX82NWXV79EQTA5QX081"

[[acceptance]]
text = "Given rect_pattern(nx, ny, dx, dy, rotation), when evaluated, then points are ordered row by row, x fastest, from (-,-) to (+,+), centred on the datum origin (geometry.md 3)"
bound = true

[[acceptance]]
text = "Given a generator, when lowered, then holes.pitch_x, pitch_y, radius, start and rotation are Members that bind and emit while derived coordinates are analysis-only (geometry.md 3; ir.md 2)"
bound = true

[[acceptance]]
text = "Given a tolerance after a pattern, when evaluated, then one independent source variable per coordinate applies; a toleranced generator member tolerances the pitch itself (geometry.md 3)"
bound = true
+++
