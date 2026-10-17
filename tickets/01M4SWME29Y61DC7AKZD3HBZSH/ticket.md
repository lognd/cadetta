+++
id = "01M4SWME29Y61DC7AKZD3HBZSH"
title = "Active configuration and custom properties read"
type = "story"
category = "in-progress"
priority = "medium"
points = 3
parent = "01M4SV2TCZM23R46EM75RYSA3B"
reporter = "lognd"
created = "2026-10-13T05:44:13Z"
updated = "2026-10-17T06:50:13Z"
persona = "design engineer"
capability = "adopt properties and configurations"
idempotency_key = "replan14:sw-props"
labels = ["creates:adapters/solidworks/src/Cadetta.SolidWorks.Core/Props/**", "creates:adapters/solidworks/src/Cadetta.SolidWorks.Com/Props/**", "creates:adapters/solidworks/tests/Cadetta.SolidWorks.Tests/Props/**"]
scope = ["adapters/solidworks/src/Cadetta.SolidWorks.Core/Props/**", "adapters/solidworks/src/Cadetta.SolidWorks.Com/Props/**", "adapters/solidworks/tests/Cadetta.SolidWorks.Tests/Props/**", "adapters/solidworks/src/Cadetta.SolidWorks.Core/Read/NativeModel.cs", "adapters/solidworks/src/Cadetta.SolidWorks.Core/Read/ObservedIrBuilder.cs"]

[[links]]
kind = "blocked-by"
target = "01M4SV2TKZD26CK9FFA1EJXSV4"

[[acceptance]]
text = "Given a file, when read, then the active configuration and custom properties become observed parameters (adapters.md 3)"
bound = false
+++
