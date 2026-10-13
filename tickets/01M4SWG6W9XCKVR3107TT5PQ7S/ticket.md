+++
id = "01M4SWG6W9XCKVR3107TT5PQ7S"
title = "Axis builtins radial, coaxial, parallel and angle_between"
type = "story"
category = "todo"
priority = "medium"
points = 8
parent = "01M4SWEQMQ8DQMN8B8GWVR4GDH"
reporter = "lognd"
created = "2026-10-13T05:41:54Z"
updated = "2026-10-13T05:41:55Z"
persona = "design engineer"
capability = "check axes without a solver"
idempotency_key = "replan14:geo-axes"
labels = ["creates:crates/cadetta-geometry/src/axes.rs"]
scope = ["crates/cadetta-geometry/src/axes.rs"]

[[links]]
kind = "blocked-by"
target = "01M4SWG0CZ9NYR3KFASKPSX4DP"

[[links]]
kind = "blocked-by"
target = "01M4SWG6N6DT76DRYYJHFB4656"

[[acceptance]]
text = "Given radial(p), when evaluated, then it is sqrt(x^2 + y^2) with the square-aware interval (geometry.md 5.1)"
bound = false

[[acceptance]]
text = "Given coaxial(a, b, allowance, over), when evaluated, then points at 0 and over on a's z are carried into b and each must have radial <= allowance, tilt included (geometry.md 5.1)"
bound = false

[[acceptance]]
text = "Given parallel(a, b, allowance, over), when evaluated, then lateral deviation over the length is within allowance (geometry.md 5.1)"
bound = false

[[acceptance]]
text = "Given angle_between(a, b), when evaluated, then it returns an Angle that is inexact and conservative (geometry.md 5.1)"
bound = false
+++
