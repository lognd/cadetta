+++
id = "01M5496CQE8FPX6F7PD9Y6EMX0"
title = "SOLIDWORKS RpcServer push dispatch and ISwSession wiring to PushService"
type = "task"
category = "in-progress"
priority = "medium"
points = 3
reporter = "lognd"
created = "2026-10-17T06:36:08Z"
updated = "2026-10-17T06:50:40Z"
scope = ["adapters/solidworks/src/Cadetta.SolidWorks.Core/Rpc/**", "adapters/solidworks/src/Cadetta.SolidWorks.Com/Session/**", "adapters/solidworks/src/Cadetta.SolidWorks.Core/Server/RpcServer.cs", "adapters/solidworks/src/Cadetta.SolidWorks.Core/Session/FakeSwSession.cs", "adapters/solidworks/src/Cadetta.SolidWorks.Core/Session/ISwHost.cs", "adapters/solidworks/src/Cadetta.SolidWorks.Core/Session/ISwSession.cs", "adapters/solidworks/src/Cadetta.SolidWorks.Core/Session/Recording", "adapters/solidworks/src/Cadetta.SolidWorks.Core/Session/SwConnection.cs", "adapters/solidworks/src/Cadetta.SolidWorks.Com/SwComSession.cs", "adapters/solidworks/src/cadetta-solidworks/Program.cs", "adapters/solidworks/src/Cadetta.SolidWorks.Tests/Server/ReadDispatchTests.cs", "adapters/solidworks/README.md"]
+++

found while working ~DKCF816: PushService/SwComPushDocument exist and are live-verified, but the push RPC method is not routed to them yet.
