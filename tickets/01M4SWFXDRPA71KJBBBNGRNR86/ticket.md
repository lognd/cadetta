+++
id = "01M4SWFXDRPA71KJBBBNGRNR86"
title = "For-placements over part arrays, D.at(point) and G007"
type = "story"
category = "in-progress"
priority = "medium"
points = 5
parent = "01M4SWEQMQ8DQMN8B8GWVR4GDH"
reporter = "lognd"
created = "2026-10-13T05:41:45Z"
updated = "2026-10-13T09:23:31Z"
persona = "design engineer"
capability = "place every instance of a part array"
idempotency_key = "replan14:geo-for"
labels = ["creates:crates/cadetta-geometry/src/placement_for.rs"]
scope = ["crates/cadetta-geometry/src/placement_for.rs", "docs/spec/geometry.md", "crates/cadetta-geometry/src/lib.rs"]

[[links]]
kind = "blocked-by"
target = "01M4ST7AKENKKS96TTXY13YMZQ"

[[links]]
kind = "blocked-by"
target = "01M4SWFXC2K6D3S93F3ER0GZZA"

[[acceptance]]
text = "Given 'place feet[i].top on base.underside.at(base.foot_sites[i]) for i in 0..count(feet)', when evaluated, then every instance is placed (geometry.md 4)"
bound = true

[[acceptance]]
text = "Given D.at(point), when evaluated, then D is offset to the point's x and y (and z for a Point3) (geometry.md 4)"
bound = true

[[acceptance]]
text = "Given a for range that leaves an instance unplaced, when evaluated, then G007 is raised (geometry.md 4, 7)"
bound = true
+++
