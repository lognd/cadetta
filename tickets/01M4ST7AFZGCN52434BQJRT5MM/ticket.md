+++
id = "01M4ST7AFZGCN52434BQJRT5MM"
title = "Arrays, Point2/Point3 and geometry patterns"
type = "story"
category = "done"
outcome = "done"
priority = "medium"
points = 5
parent = "01M4SPWGB2729B40CQE81MHM2Z"
reporter = "lognd"
created = "2026-10-13T05:02:06Z"
updated = "2026-10-13T08:39:23Z"
persona = "design engineer"
capability = "compute hole patterns as points without trigonometry by hand"
idempotency_key = "replan13:Arrays, Point2/Point3 and geometry patterns"
labels = ["creates:crates/cadetta-sema/src/{array,point}.rs"]
scope = ["crates/cadetta-sema/src/{array,point}.rs", "crates/cadetta-sema/src/eval.rs", "crates/cadetta-sema/src/func.rs", "docs/spec/language.md"]

[[links]]
kind = "blocks"
target = "01M4ST7AKENKKS96TTXY13YMZQ"

[[acceptance]]
text = "Given `[a, b, c]` and `[T; N]` parameters, when evaluated, then fixed arrays hold N elements of one kind (language.md 3.3, 3.8)"
bound = true

[[acceptance]]
text = "Given `Point2(x, y)`, `p + q`, `p - q` and `p * s`, when evaluated, then results are Point2 with Length members `.x .y` (language.md 3.8)"
bound = true

[[acceptance]]
text = "Given `rect_pattern(2, 2, 10mm, 10mm)`, when evaluated, then four points are centred on the origin, rows first (language.md 3.8)"
bound = true

[[acceptance]]
text = "Given `circle_pattern(n, r, start)` and `distance(p, q)`, when evaluated, then n points and a Length result (language.md 3.8)"
bound = true

[[acceptance]]
text = "Given `holes + Point2(5mm, 0mm)`, when evaluated, then the point is applied to each element (language.md 3.8)"
bound = true

[[acceptance]]
text = "Given Point2/Point3 arrays, when this ticket lands, then datum typing, G001 and generator members are left to the geometry epic stories (geometry.md 3)"
bound = true
+++
