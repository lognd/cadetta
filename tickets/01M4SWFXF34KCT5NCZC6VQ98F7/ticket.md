+++
id = "01M4SWFXF34KCT5NCZC6VQ98F7"
title = "Placement op Members and canonical paths"
type = "story"
category = "in-progress"
priority = "medium"
points = 3
parent = "01M4SWEQMQ8DQMN8B8GWVR4GDH"
reporter = "lognd"
created = "2026-10-13T05:41:45Z"
updated = "2026-10-13T09:14:27Z"
persona = "design engineer"
capability = "bind placement offsets to mates"
idempotency_key = "replan14:geo-opmembers"
labels = ["creates:crates/cadetta-geometry/src/op_members.rs"]
scope = ["crates/cadetta-geometry/src/op_members.rs", "crates/cadetta-geometry/src/lib.rs", "docs/spec/geometry.md"]

[[links]]
kind = "blocked-by"
target = "01M4SWFXC2K6D3S93F3ER0GZZA"

[[acceptance]]
text = "Given placement ops, when lowered, then each argument is a Member with path Assembly/part.place.<op index>.<axis> and provenance authored (geometry.md 4; ir.md 2)"
bound = true

[[acceptance]]
text = "Given a for-placement, when lowered, then the path is Assembly/part[i].place.<op>.<axis> (geometry.md 4; ir.md 2)"
bound = true
+++
