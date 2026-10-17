+++
id = "01M4SV2TKZD26CK9FFA1EJXSV4"
title = "SOLIDWORKS read to observed IR"
type = "story"
category = "in-progress"
priority = "medium"
points = 8
parent = "01M4SV2TCZM23R46EM75RYSA3B"
reporter = "lognd"
created = "2026-10-13T05:17:07Z"
updated = "2026-10-17T06:32:52Z"
persona = "design engineer"
capability = "read globals, equations, dimensions and custom properties"
labels = ["creates:adapters/solidworks/src/Cadetta.SolidWorks.Core/Read/**", "creates:adapters/solidworks/src/Cadetta.SolidWorks.Com/Read/**", "creates:adapters/solidworks/tests/Cadetta.SolidWorks.Tests/Read/**"]
scope = ["adapters/solidworks/src/Cadetta.SolidWorks.Core/Read/**", "adapters/solidworks/src/Cadetta.SolidWorks.Com/Read/**", "adapters/solidworks/tests/Cadetta.SolidWorks.Tests/Read/**", "adapters/solidworks/src/Cadetta.SolidWorks.Tests/Read/**", "adapters/solidworks/src/cadetta-solidworks/Program.cs", "adapters/solidworks/src/Cadetta.SolidWorks.Com/SwComSession.cs", "adapters/solidworks/API-SURFACE.md", "adapters/solidworks/DEVELOPING.md", "docs/spec/solidworks.md"]

[[links]]
kind = "blocked-by"
target = "01M4ST7B09TGN6VSXJBZ8FDVCN"

[[links]]
kind = "blocked-by"
target = "01M4ST7B3CYJNHGNCCQ3EYW9WC"

[[links]]
kind = "blocked-by"
target = "01M4SV2TGHW85Z0PZS8QHBFB4S"

[[links]]
kind = "blocked-by"
target = "01M4SV3AAC44Q1DCE44GG3XB6W"

[[links]]
kind = "blocked-by"
target = "01M4VB640810PZEZHQJ1T5WFTK"

[[links]]
kind = "blocked-by"
target = "01M4VH6XQF4GRQM20G0Z9SWC8W"

[[acceptance]]
text = "Given a part with equation-manager globals and equations, when read, then observed IR holds each with unit_assumed from the native unit (adapters.md 3)"
bound = true

[[acceptance]]
text = "Given a dimension D1@Feature@File, when read, then it is observed with its locator and kind dimension (adapters.md 3)"
bound = true

[[acceptance]]
text = "Given custom properties, when read, then they are observed with kind property (adapters.md 3)"
bound = true

[[acceptance]]
text = "Given a driven or unextractable value, when read, then exact:false with a reason (adapters.md 4 rule 4)"
bound = true

[[acceptance]]
text = "Given a part or assembly, when read, then dimension equations, dimension tolerances, coordinate-system transforms (frames[]) and component transforms with reference properties (instances[]) are in observed IR (ir.md 1; solidworks.md 4)"
bound = true
+++
