+++
id = "01M4SV2TVM50WJRZ42ZHXVD96N"
title = "SOLIDWORKS regen with rebuild-error mapping to R004"
type = "story"
category = "in-progress"
priority = "medium"
points = 5
parent = "01M4SV2TCZM23R46EM75RYSA3B"
reporter = "lognd"
created = "2026-10-13T05:17:07Z"
updated = "2026-10-17T06:59:24Z"
persona = "design engineer"
capability = "regenerate headless and see rebuild failures as diagnostics"
labels = ["creates:adapters/solidworks/src/Cadetta.SolidWorks.Core/Regen/**", "creates:adapters/solidworks/src/Cadetta.SolidWorks.Com/Regen/**", "creates:adapters/solidworks/tests/Cadetta.SolidWorks.Tests/Regen/**"]
scope = ["adapters/solidworks/src/Cadetta.SolidWorks.Core/Regen/**", "adapters/solidworks/src/Cadetta.SolidWorks.Com/Regen/**", "adapters/solidworks/tests/Cadetta.SolidWorks.Tests/Regen/**", "adapters/solidworks/src/Cadetta.SolidWorks.Core/Server/RpcServer.cs", "adapters/solidworks/src/cadetta-solidworks/Program.cs", "adapters/solidworks/src/Cadetta.SolidWorks.Com/SwComSession.cs", "adapters/solidworks/src/Cadetta.SolidWorks.Tests/Regen/**", "adapters/solidworks/DEVELOPING.md"]

[[links]]
kind = "blocked-by"
target = "01M4SPWRAWXJTWT4G08578RNQM"

[[links]]
kind = "blocked-by"
target = "01M4SV2TR8ENTA87J1DDKCF816"

[[links]]
kind = "blocked-by"
target = "01M4SV3AAC44Q1DCE44GG3XB6W"

[[acceptance]]
text = "Given a rebuild error, when regen runs, then R004 regen-failed is returned naming the feature (adapters.md 5, lints.md R004)"
bound = true

[[acceptance]]
text = "Given a session with unsaved user changes, when driving, then the adapter refuses and reports (OQ-8)"
bound = true

[[acceptance]]
text = "Given a regen failure, when pushing, then R004 is reported and atomic restore runs (adapters.md 2.3)"
bound = true
+++
