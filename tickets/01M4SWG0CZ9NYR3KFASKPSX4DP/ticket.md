+++
id = "01M4SWG0CZ9NYR3KFASKPSX4DP"
title = "Square-aware intervals for x^2 and x*x"
type = "story"
category = "in-progress"
priority = "medium"
points = 5
parent = "01M4SWEQMQ8DQMN8B8GWVR4GDH"
reporter = "lognd"
created = "2026-10-13T05:41:45Z"
updated = "2026-10-13T06:27:20Z"
persona = "design engineer"
capability = "get defined radial distances"
idempotency_key = "replan14:geo-square"
labels = ["creates:crates/cadetta-units/src/square.rs"]
scope = ["crates/cadetta-units/src/square.rs", "crates/cadetta-units/src/tolerance.rs"]

[[links]]
kind = "blocked-by"
target = "01M4SPWNNDNT60Z38K24AEKVAE"

[[links]]
kind = "blocked-by"
target = "01M4SPWNVX14PTZ4B19436P66Q"

[[acceptance]]
text = "Given x ^ 2 or x * x of the same affine form, when evaluated, then the interval is never negative (quantities.md 5.4; geometry.md 5.1)"
bound = false

[[acceptance]]
text = "Given sqrt(x^2 + y^2) with x straddling zero, when evaluated, then it stays defined and is marked conservative (quantities.md 5.4)"
bound = false

[[acceptance]]
text = "Given x * y of two distinct forms, when evaluated, then the ordinary conservative interval applies (quantities.md 5.4)"
bound = false
+++
