+++
id = "01M4SWFX4ZH1Q8BJPEEWYEKQJC"
title = "Datum transforms: exact 90deg multiples, inexact otherwise, toleranced angles"
type = "story"
category = "done"
outcome = "done"
priority = "medium"
points = 8
parent = "01M4SWEQMQ8DQMN8B8GWVR4GDH"
reporter = "lognd"
created = "2026-10-13T05:41:44Z"
updated = "2026-10-13T08:31:53Z"
persona = "design engineer"
capability = "compose datum transforms exactly"
idempotency_key = "replan14:geo-transform"
labels = ["creates:crates/cadetta-geometry/src/transform.rs"]
scope = ["crates/cadetta-geometry/src/transform.rs", "crates/cadetta-geometry/Cargo.toml", "crates/cadetta-geometry/src/lib.rs", "docs/spec/geometry.md"]

[[links]]
kind = "blocked-by"
target = "01M4SPWNNDNT60Z38K24AEKVAE"

[[links]]
kind = "blocked-by"
target = "01M4SPWNVX14PTZ4B19436P66Q"

[[links]]
kind = "blocked-by"
target = "01M4SWFX374CNSPDDD289Z9BCS"

[[acceptance]]
text = "Given untoleranced rotations by whole multiples of 90deg, when composed, then the 3x4 transform is Exact (geometry.md 2; ir.md 3)"
bound = true

[[acceptance]]
text = "Given any other angle, when composed, then the transform is Inexact with a root and Q006 applies to the nominal (geometry.md 2; quantities.md 1)"
bound = true

[[acceptance]]
text = "Given a toleranced angle, when a coordinate is carried through it, then the coordinate is conservative (geometry.md 2; quantities.md 5.4)"
bound = true

[[acceptance]]
text = "Given offsets with tolerances, when composed, then they remain source variables of the affine form (geometry.md 2; quantities.md 5.4)"
bound = true
+++
