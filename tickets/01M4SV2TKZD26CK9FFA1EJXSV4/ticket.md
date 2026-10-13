+++
id = "01M4SV2TKZD26CK9FFA1EJXSV4"
title = "SOLIDWORKS read to observed IR"
type = "story"
category = "todo"
priority = "medium"
points = 8
parent = "01M4SV2TCZM23R46EM75RYSA3B"
reporter = "lognd"
created = "2026-10-13T05:17:07Z"
updated = "2026-10-13T05:17:07Z"
persona = "design engineer"
capability = "read globals, equations, dimensions and custom properties"
labels = ["creates:adapters/solidworks/src/read.py"]
scope = ["adapters/solidworks/src/read.py"]

[[acceptance]]
text = "Given a part with equation-manager globals and equations, when read, then observed IR holds each with unit_assumed from the native unit (adapters.md 3)"
bound = false

[[acceptance]]
text = "Given a dimension D1@Feature@File, when read, then it is observed with its locator and kind dimension (adapters.md 3)"
bound = false

[[acceptance]]
text = "Given custom properties, when read, then they are observed with kind property (adapters.md 3)"
bound = false

[[acceptance]]
text = "Given a driven or unextractable value, when read, then exact:false with a reason (adapters.md 4 rule 4)"
bound = false
+++
