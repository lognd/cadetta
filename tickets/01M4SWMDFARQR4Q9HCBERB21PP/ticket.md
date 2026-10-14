+++
id = "01M4SWMDFARQR4Q9HCBERB21PP"
title = "Session attach and hidden start"
type = "story"
category = "done"
outcome = "done"
priority = "medium"
points = 5
parent = "01M4SV2TCZM23R46EM75RYSA3B"
reporter = "lognd"
created = "2026-10-13T05:44:12Z"
updated = "2026-10-14T21:22:11Z"
persona = "design engineer"
capability = "drive SOLIDWORKS unattended"
idempotency_key = "replan14:sw-session"
labels = ["creates:adapters/solidworks/src/Cadetta.SolidWorks.Core/Session/**", "creates:adapters/solidworks/src/Cadetta.SolidWorks.Com/Session/**", "creates:adapters/solidworks/tests/Cadetta.SolidWorks.Tests/Session/**"]
scope = ["adapters/solidworks/src/Cadetta.SolidWorks.Core/Session/**", "adapters/solidworks/src/Cadetta.SolidWorks.Com/Session/**", "adapters/solidworks/tests/Cadetta.SolidWorks.Tests/Session/**", "adapters/solidworks/src/Cadetta.SolidWorks.Com/SwComHost.cs", "adapters/solidworks/src/Cadetta.SolidWorks.Com/SwComSession.cs", "adapters/solidworks/src/Cadetta.SolidWorks.Tests/Session/**", "adapters/solidworks/src/cadetta-solidworks/Program.cs", "adapters/solidworks/README.md", "changelog.d/01M4SWMDFARQR4Q9HCBERB21PP.added.md"]

[[links]]
kind = "blocked-by"
target = "01M4SV2TGHW85Z0PZS8QHBFB4S"

[[links]]
kind = "blocked-by"
target = "01M4SV3AAC44Q1DCE44GG3XB6W"

[[links]]
kind = "blocked-by"
target = "01M4SWMCFYWCHWRQM0B6SFGK71"

[[acceptance]]
text = "Given a running SOLIDWORKS, when open runs, then the adapter attaches; otherwise it starts one hidden (adapters.md 2.2)"
bound = true
+++
