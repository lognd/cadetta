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
updated = "2026-10-13T06:05:27Z"
persona = "design engineer"
capability = "set project-wide fastener policy once"
idempotency_key = "replan13:FastenerPolicy, IsoDefault and the default attribute"
labels = ["creates:std/src/fasteners/policy.cdt"]
scope = ["std/src/fasteners/policy.cdt"]

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

[[acceptance]]
text = "Given std::fasteners::IsoDefault, when read, then engagement factors and tap columns come from Materials, lengths from Iso4762Len, drills from the ISO 2306 preferred series, bottom_clearance = 1mm, tap_lead_in = 3 (pitches, plug tap) and drill_point = 0.3 (118deg), each cited to Machinery Handbook (stdlib.md 4)"
bound = false

[[acceptance]]
text = "Given precedence of a component #![default], module #![default], package [defaults], then the std default IsoDefault, when no override exists, then IsoDefault applies and no lint fires on the fallback (stdlib.md 4)"
bound = false
+++
