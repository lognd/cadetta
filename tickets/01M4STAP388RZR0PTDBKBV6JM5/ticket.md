+++
id = "01M4STAP388RZR0PTDBKBV6JM5"
title = "Metric fastener tables"
type = "story"
category = "todo"
priority = "medium"
points = 8
parent = "01M4STANRAWHV180C7RH0NQAY9"
reporter = "lognd"
created = "2026-10-13T05:03:56Z"
updated = "2026-10-13T05:03:56Z"
persona = "design engineer"
capability = "look up thread, head and nut dimensions"
idempotency_key = "replan13:Metric fastener tables"
labels = ["creates:std/fasteners/tables.cdt"]
scope = ["std/fasteners/tables.cdt"]

[[acceptance]]
text = "Given MetricThread M1_6 to M20, Thread, Iso273, TapDrill, Iso4762Head, Iso4762Len and Iso4032, when loaded, then every value is as published and cited (stdlib.md 4)"
bound = false

[[acceptance]]
text = "Given tolerances, when read, then they are kept as published (stdlib.md 4)"
bound = false
+++
