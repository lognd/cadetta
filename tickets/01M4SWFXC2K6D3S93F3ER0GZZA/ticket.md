+++
id = "01M4SWFXC2K6D3S93F3ER0GZZA"
title = "Placement tree: place A on B, composition, G002 and G006"
type = "story"
category = "in-progress"
priority = "medium"
points = 8
parent = "01M4SWEQMQ8DQMN8B8GWVR4GDH"
reporter = "lognd"
created = "2026-10-13T05:41:44Z"
updated = "2026-10-13T08:46:43Z"
persona = "design engineer"
capability = "place parts in a tree"
idempotency_key = "replan14:geo-place"
labels = ["creates:crates/cadetta-geometry/src/placement.rs"]
scope = ["crates/cadetta-geometry/src/placement.rs", "docs/spec/geometry.md", "crates/cadetta-geometry/src/lib.rs"]

[[links]]
kind = "blocked-by"
target = "01M4ST7AKENKKS96TTXY13YMZQ"

[[links]]
kind = "blocked-by"
target = "01M4SWFX374CNSPDDD289Z9BCS"

[[links]]
kind = "blocked-by"
target = "01M4SWFX4ZH1Q8BJPEEWYEKQJC"

[[acceptance]]
text = "Given 'place bracket.base on rail.site', when evaluated, then datum frames coincide, z directions included (geometry.md 4)"
bound = true

[[acceptance]]
text = "Given a chain that returns to a placed part, when evaluated, then G002 is raised (geometry.md 4, 7)"
bound = true

[[acceptance]]
text = "Given a part placed twice, or overlapping for ranges, when evaluated, then G006 is raised (geometry.md 4, 7)"
bound = true

[[acceptance]]
text = "Given a subassembly, when placed, then its parts come along and instance paths compose as Mount/sub/bracket.holes[1] (geometry.md 4)"
bound = true

[[acceptance]]
text = "Given variants, when a placement member differs per alternate, then it is re-evaluated like any member (geometry.md 4)"
bound = true
+++
