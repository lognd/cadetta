+++
id = "01M4SWG6N6DT76DRYYJHFB4656"
title = "Re-expression 'p in D' and G003"
type = "story"
category = "in-progress"
priority = "medium"
points = 5
parent = "01M4SWEQMQ8DQMN8B8GWVR4GDH"
reporter = "lognd"
created = "2026-10-13T05:41:54Z"
updated = "2026-10-13T13:42:10Z"
persona = "design engineer"
capability = "compare positions across parts"
idempotency_key = "replan14:geo-in"
labels = ["creates:crates/cadetta-geometry/src/reexpress.rs"]
scope = ["crates/cadetta-geometry/src/reexpress.rs"]

[[links]]
kind = "blocked-by"
target = "01M4SWFX4ZH1Q8BJPEEWYEKQJC"

[[links]]
kind = "blocked-by"
target = "01M4SWFX82NWXV79EQTA5QX081"

[[links]]
kind = "blocked-by"
target = "01M4SWFXC2K6D3S93F3ER0GZZA"

[[acceptance]]
text = "Given p in D inside a part, when evaluated, then p is carried through the component's own datums (geometry.md 3)"
bound = false

[[acceptance]]
text = "Given an assembly, when p in D crosses parts, then it composes the placements on the path (geometry.md 3, 4)"
bound = false

[[acceptance]]
text = "Given a conversion out of an unplaced part, or re-typing where the model lacks the part or its placement, when evaluated, then G003 is raised (geometry.md 3, 4, 7)"
bound = false

[[acceptance]]
text = "Given an array, when mapped with in, then every element is converted; a Point2 on d becomes a Point3 at z = 0 of d (geometry.md 3)"
bound = false
+++
