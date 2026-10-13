+++
id = "01M4STAP69F2SZMD1C8HBVSAZC"
title = "Screw interface and Iso4762 family"
type = "story"
category = "todo"
priority = "medium"
points = 3
parent = "01M4STANRAWHV180C7RH0NQAY9"
reporter = "lognd"
created = "2026-10-13T05:03:56Z"
updated = "2026-10-13T05:05:21Z"
persona = "design engineer"
capability = "use a socket head cap screw as a Screw"
idempotency_key = "replan13:Screw interface and Iso4762 family"
labels = ["creates:std/fasteners/screw.cdt"]
scope = ["std/fasteners/screw.cdt"]

[[links]]
kind = "blocks"
target = "01M4STAP93AYTZDJYWCCAS1TRH"

[[acceptance]]
text = "Given the Screw interface, when Iso4762 is checked, then it implements it (stdlib.md 2)"
bound = false

[[acceptance]]
text = "Given an Iso4762 size and length, when out of Iso4762Len, then lookup errors (stdlib.md 2)"
bound = false
+++
