+++
id = "01M4SWG6Z6JVP5Z7H38HWJP0GQ"
title = "ScrewJoint float, locating and the implicit placement float"
type = "story"
category = "done"
outcome = "done"
priority = "medium"
points = 8
parent = "01M4SWEQMQ8DQMN8B8GWVR4GDH"
reporter = "lognd"
created = "2026-10-13T05:41:54Z"
updated = "2026-10-14T20:58:53Z"
persona = "design engineer"
capability = "avoid false 'holds' answers"
idempotency_key = "replan14:geo-float"
labels = ["creates:crates/cadetta-geometry/src/float.rs"]
scope = ["crates/cadetta-geometry/src/float.rs", "crates/cadetta-geometry/src/placement.rs", "crates/cadetta-geometry/src/lib.rs", "docs/spec/geometry.md", "docs/spec/stdlib.md", "changelog.d/**"]

[[links]]
kind = "blocked-by"
target = "01M4SWFXC2K6D3S93F3ER0GZZA"

[[links]]
kind = "blocked-by"
target = "01M4SWG6XQ2M0GR620Q6MJE3XF"

[[links]]
kind = "blocked-by"
target = "01M4VB636BKBDGRW3NBMGK7CBZ"

[[links]]
kind = "blocked-by"
target = "01M4VQXPDA4TEKE455FNYB5TXC"

[[acceptance]]
text = "Given a ScrewJoint, when evaluated, then it exports float: Length equal to the allowance its aligned check uses (already met by std/src/fasteners/screwjoint.cdt; checked by tests/conformance/stdlib/joint-float-nut-sum; geometry.md 5.2)"
bound = true

[[acceptance]]
text = "Given a placement edge with non-locating joints between parent and child supplied as EdgeFloat, when placed by cadetta-geometry, then the edge carries [+-float] on x and y and a rotation [+-theta] about z through the onto centroid, theta = 2 * asin(float / s), 180deg when float >= s (float.rs; geometry.md 5.2)"
bound = true

[[acceptance]]
text = "Given a one-point pattern or s = 0, when evaluated in cadetta-geometry, then theta is 180deg and float::unconstrained_rotation (the G008 condition) is true"
bound = true

[[acceptance]]
text = "Given several joints on one edge, when selected, then the edge takes the whole bound of the joint with the smallest theta, then smallest float, then first canonical path; any locating joint removes the float (float::select)"
bound = true

[[acceptance]]
text = "Given FloatMode::Without(path), when placing, then that joint's own edge is read without its float (place_assembly_reading)"
bound = true
+++
