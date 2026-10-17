+++
id = "01M548Y09E8W1NVTJSSS0GDS2T"
title = "Wire read method to ReadService in SOLIDWORKS RpcServer"
type = "task"
category = "in-progress"
priority = "medium"
points = 2
reporter = "lognd"
created = "2026-10-17T06:31:33Z"
updated = "2026-10-17T06:41:06Z"
scope = ["adapters/solidworks/src/Cadetta.SolidWorks.Core/Server/**", "adapters/solidworks/src/Cadetta.SolidWorks.Tests/Server/**", "adapters/solidworks/DEVELOPING.md", "adapters/solidworks/README.md"]
+++

found while working ~1EJXSV4: ReadService exists (Core/Read) and SwComSession.ReadObserved works on real SOLIDWORKS, but RpcServer does not dispatch the read method yet.
