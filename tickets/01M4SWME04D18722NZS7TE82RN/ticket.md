+++
id = "01M4SWME04D18722NZS7TE82RN"
title = "Assembly read, R011 and component field"
type = "story"
category = "in-progress"
priority = "medium"
points = 5
parent = "01M4SV2TCZM23R46EM75RYSA3B"
reporter = "lognd"
created = "2026-10-13T05:44:13Z"
updated = "2026-10-17T06:38:39Z"
persona = "design engineer"
capability = "bind assembly instances"
idempotency_key = "replan14:sw-asm"
labels = ["creates:adapters/solidworks/src/Cadetta.SolidWorks.Core/Assembly/**", "creates:adapters/solidworks/src/Cadetta.SolidWorks.Com/Assembly/**", "creates:adapters/solidworks/tests/Cadetta.SolidWorks.Tests/Assembly/**"]
scope = ["adapters/solidworks/src/Cadetta.SolidWorks.Core/Assembly/**", "adapters/solidworks/src/Cadetta.SolidWorks.Com/Assembly/**", "adapters/solidworks/tests/Cadetta.SolidWorks.Tests/Assembly/**", "adapters/solidworks/src/Cadetta.SolidWorks.Tests/Assembly/**", "adapters/solidworks/src/Cadetta.SolidWorks.Core/Read/NativeModel.cs", "adapters/solidworks/src/Cadetta.SolidWorks.Core/Read/ObservedIrBuilder.cs", "adapters/solidworks/src/Cadetta.SolidWorks.Core/Read/ReadService.cs"]

[[links]]
kind = "blocked-by"
target = "01M4SV2TKZD26CK9FFA1EJXSV4"

[[acceptance]]
text = "Given an assembly, when read, then locators carry component and R011 fires when a slot holds a file other than its primary (adapters.md 3; lints.md 3 R)"
bound = false
+++
