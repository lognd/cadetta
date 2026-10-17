+++
id = "01M5496CQE8FPX6F7PD9Y6EMX0"
title = "SOLIDWORKS RpcServer push dispatch and ISwSession wiring to PushService"
type = "task"
category = "in-progress"
priority = "medium"
points = 3
reporter = "lognd"
created = "2026-10-17T06:36:08Z"
updated = "2026-10-17T06:49:20Z"
scope = ["adapters/solidworks/src/Cadetta.SolidWorks.Core/Rpc/**", "adapters/solidworks/src/Cadetta.SolidWorks.Com/Session/**", "adapters/solidworks/src/Cadetta.SolidWorks.Core/Server/RpcServer.cs", "adapters/solidworks/src/Cadetta.SolidWorks.Core/Session/FakeSwSession.cs"]
+++

found while working ~DKCF816: PushService/SwComPushDocument exist and are live-verified, but the push RPC method is not routed to them yet.
