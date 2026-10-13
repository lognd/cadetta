+++
id = "01M4SWG6Z6JVP5Z7H38HWJP0GQ"
title = "ScrewJoint float, locating and the implicit placement float"
type = "story"
category = "todo"
priority = "medium"
points = 5
parent = "01M4SWEQMQ8DQMN8B8GWVR4GDH"
reporter = "lognd"
created = "2026-10-13T05:41:54Z"
updated = "2026-10-13T05:41:55Z"
persona = "design engineer"
capability = "avoid false 'holds' answers"
idempotency_key = "replan14:geo-float"
labels = ["creates:std/fasteners/screwjoint_float.cdt", "creates:crates/cadetta-geometry/src/float.rs"]
scope = ["std/fasteners/screwjoint_float.cdt", "crates/cadetta-geometry/src/float.rs"]

[[links]]
kind = "blocked-by"
target = "01M4SWG6XQ2M0GR620Q6MJE3XF"

[[acceptance]]
text = "Given a ScrewJoint, when evaluated, then it exports float: Length equal to the allowance its aligned check uses (geometry.md 5.2; stdlib.md 3)"
bound = false

[[acceptance]]
text = "Given at/onto, a clamped part placed on its host's pattern datum and locating = false, when evaluated, then the placement carries [+-float] on x and y as one source-variable pair per joint (geometry.md 5.2)"
bound = false

[[acceptance]]
text = "Given the joint's own aligned check, when evaluated, then it reads the placement without the float (geometry.md 5.2)"
bound = false

[[acceptance]]
text = "Given locating = true, when evaluated, then no float is added (geometry.md 5.2; stdlib.md 3)"
bound = false
+++
