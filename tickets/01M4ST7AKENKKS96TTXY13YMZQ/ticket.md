+++
id = "01M4ST7AKENKKS96TTXY13YMZQ"
title = "Part arrays, count and roll-ups"
type = "story"
category = "todo"
priority = "medium"
points = 5
parent = "01M4SPWGB2729B40CQE81MHM2Z"
reporter = "lognd"
created = "2026-10-13T05:02:06Z"
updated = "2026-10-13T05:02:06Z"
persona = "design engineer"
capability = "sum mass, cost and lead time over every part instance"
idempotency_key = "replan13:Part arrays, count and roll-ups"
labels = ["creates:crates/cadetta-sema/src/{parts,rollup}.rs"]
scope = ["crates/cadetta-sema/src/{parts,rollup}.rs"]

[[acceptance]]
text = "Given `part wheel[4]: Caster`, when evaluated, then count(wheel) is a Count of 4 and `wheel[i]` is an instance; N is evaluated before any member (language.md 3.5)"
bound = false

[[acceptance]]
text = "Given `wheel.hub_bore` without an index, when read, then it equals `wheel[0].hub_bore` (language.md 3.5)"
bound = false

[[acceptance]]
text = "Given `sum(name.member)`, `sum(self, mass)` and `max(self, lead_time)`, when evaluated, then every instance counts, recursively (language.md 3.5)"
bound = false

[[acceptance]]
text = "Given a part lacking the parameter, when rolled up without `?`, then an error; with `sum?(self, mass)` the missing parts are listed in the result note (language.md 3.5)"
bound = false

[[acceptance]]
text = "Given parameterized components with defaults, when instantiated, then optional arguments apply and enum arguments may be bare variants (language.md 3.1)"
bound = false
+++
