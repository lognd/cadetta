+++
id = "01M4SWG6Z6JVP5Z7H38HWJP0GQ"
title = "ScrewJoint float, locating and the implicit placement float"
type = "story"
category = "todo"
priority = "medium"
points = 8
parent = "01M4SWEQMQ8DQMN8B8GWVR4GDH"
reporter = "lognd"
created = "2026-10-13T05:41:54Z"
updated = "2026-10-14T01:24:52Z"
persona = "design engineer"
capability = "avoid false 'holds' answers"
idempotency_key = "replan14:geo-float"
labels = ["creates:std/fasteners/screwjoint_float.cdt", "creates:crates/cadetta-geometry/src/float.rs"]
scope = ["std/fasteners/screwjoint_float.cdt", "crates/cadetta-geometry/src/float.rs", "crates/cadetta-lint/src/rules/**", "tests/conformance/geometry/**"]

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
text = "Given a ScrewJoint, when evaluated, then it exports float: Length equal to the allowance its aligned check uses (geometry.md 5.2; stdlib.md 3)"
bound = false

[[acceptance]]
text = "Given a non-locating joint whose two parts are placement parent and child in either order, whatever datum the placement names, when evaluated, then that edge carries [+-float] on x and y and a rotation [+-theta] about z through the onto centroid, theta = 2 * asin(float / s), 180deg when float >= s (geometry.md 5.2)"
bound = false

[[acceptance]]
text = "Given a one-point pattern or s = 0, when evaluated, then theta is 180deg, checks depending on the rotation are undecided and G008 fires (geometry.md 5.2; lints.md G008)"
bound = false

[[acceptance]]
text = "Given several non-locating joints on one edge, when evaluated, then the edge takes the whole bound of the joint with the smallest theta, then smallest float, then first canonical path; any locating joint removes the float (geometry.md 5.2)"
bound = false

[[acceptance]]
text = "Given the joint's own aligned check, when evaluated, then it reads its edge without the float; joints between parts that are not parent and child add no float (geometry.md 5.2)"
bound = false
+++
