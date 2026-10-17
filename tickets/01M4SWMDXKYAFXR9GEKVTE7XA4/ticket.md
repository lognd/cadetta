+++
id = "01M4SWMDXKYAFXR9GEKVTE7XA4"
title = "Frame locators, geometry bindings and R007"
type = "story"
category = "in-progress"
priority = "medium"
points = 8
parent = "01M4SV2TCZM23R46EM75RYSA3B"
reporter = "lognd"
created = "2026-10-13T05:44:12Z"
updated = "2026-10-17T06:20:42Z"
persona = "design engineer"
capability = "drive sketches from datums"
idempotency_key = "replan14:sw-frame"
labels = ["creates:adapters/solidworks/src/Cadetta.SolidWorks.Core/Frames/**", "creates:adapters/solidworks/src/Cadetta.SolidWorks.Com/Frames/**", "creates:adapters/solidworks/tests/Cadetta.SolidWorks.Tests/Frames/**"]
scope = ["adapters/solidworks/src/Cadetta.SolidWorks.Core/Frames/**", "adapters/solidworks/src/Cadetta.SolidWorks.Com/Frames/**", "adapters/solidworks/tests/Cadetta.SolidWorks.Tests/Frames/**", "adapters/solidworks/src/Cadetta.SolidWorks.Tests/Cadetta.SolidWorks.Tests.csproj", "adapters/solidworks/README.md"]

[[links]]
kind = "blocked-by"
target = "01M4SWMCYW919R5JKSV2T1ZYBN"

[[links]]
kind = "blocked-by"
target = "01M4SWMDFARQR4Q9HCBERB21PP"

[[links]]
kind = "blocked-by"
target = "01M4VH6XQF4GRQM20G0Z9SWC8W"

[[acceptance]]
text = "Given a datum-plane sketch coordinate, when bound, then the dimension is pushed with sign and sketch-origin rules (geometry.md 6)"
bound = false

[[acceptance]]
text = "Given a sketch not on the datum plane or not dimensioned from its origin, when read, then R007 is raised (lints.md 3 R)"
bound = false

[[acceptance]]
text = "Given a datum bound to a SOLIDWORKS coordinate system, when read, then its transform is reported in frames[] and, under datums = create, the coordinate system is created and updated from numeric values (solidworks.md 4)"
bound = false
+++
