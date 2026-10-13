+++
id = "01M4STAP388RZR0PTDBKBV6JM5"
title = "Metric fastener tables"
type = "story"
category = "in-progress"
priority = "medium"
points = 8
parent = "01M4STANRAWHV180C7RH0NQAY9"
reporter = "lognd"
created = "2026-10-13T05:03:56Z"
updated = "2026-10-13T07:46:42Z"
persona = "design engineer"
capability = "look up thread, head and nut dimensions"
idempotency_key = "replan13:Metric fastener tables"
labels = ["creates:std/src/fasteners/tables.cdt"]
scope = ["std/src/fasteners/tables.cdt", "crates/cadetta-syntax/tests/std_fasteners_tables.rs", "changelog.d/01M4STAP388RZR0PTDBKBV6JM5*", "docs/spec/stdlib.md"]

[[links]]
kind = "blocks"
target = "01M4STAP69F2SZMD1C8HBVSAZC"

[[acceptance]]
text = "Given MetricThread M1_6 to M20, Thread, Iso273, TapDrill, Iso4762Head, Iso4762Len and Iso4032, when loaded, then every value is as published and cited (stdlib.md 2)"
bound = true

[[acceptance]]
text = "Given tolerances, when read, then they are kept as published (stdlib.md 2)"
bound = true
+++
