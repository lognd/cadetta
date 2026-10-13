+++
id = "01M4STAP93AYTZDJYWCCAS1TRH"
title = "FastenerPolicy, IsoDefault and the default attribute"
type = "story"
category = "todo"
priority = "medium"
points = 5
parent = "01M4STANRAWHV180C7RH0NQAY9"
reporter = "lognd"
created = "2026-10-13T05:03:56Z"
updated = "2026-10-13T05:45:12Z"
persona = "design engineer"
capability = "set project-wide fastener policy once"
idempotency_key = "replan13:FastenerPolicy, IsoDefault and the default attribute"
labels = ["creates:std/fasteners/policy.cdt"]
scope = ["std/fasteners/policy.cdt"]

[[links]]
kind = "blocks"
target = "01M4STAPCEF5EPEYX0MZDSBA7J"

[[acceptance]]
text = "Given FastenerPolicy and IsoDefault, when used with `#![default(FastenerPolicy = IsoDefault)]`, then ScrewJoint policy fills from it (stdlib.md 4, language.md 3.11)"
bound = false

[[acceptance]]
text = "Given policy tables, when read, then factors and TapColumn cut/form are typed (stdlib.md 4)"
bound = false

[[acceptance]]
text = "Given FastenerPolicy, when no default is set, then std supplies it and project [defaults] may override (stdlib.md 4; config-cli.md 2)"
bound = false
+++
