+++
id = "01M4SWMDHYG9RMHHTMBTA4NT7M"
title = "Busy and modal detection (R009)"
type = "story"
category = "todo"
priority = "medium"
points = 3
parent = "01M4SV2TCZM23R46EM75RYSA3B"
reporter = "lognd"
created = "2026-10-13T05:44:12Z"
updated = "2026-10-15T04:30:06Z"
persona = "design engineer"
capability = "avoid corrupting an interactive session"
idempotency_key = "replan14:sw-busy"
labels = ["creates:adapters/solidworks/src/Cadetta.SolidWorks.Core/Busy/**", "creates:adapters/solidworks/src/Cadetta.SolidWorks.Com/Busy/**", "creates:adapters/solidworks/tests/Cadetta.SolidWorks.Tests/Busy/**"]
scope = ["adapters/solidworks/src/Cadetta.SolidWorks.Core/Busy/**", "adapters/solidworks/src/Cadetta.SolidWorks.Com/Busy/**", "adapters/solidworks/tests/Cadetta.SolidWorks.Tests/Busy/**"]

[[links]]
kind = "blocked-by"
target = "01M4SWMDFARQR4Q9HCBERB21PP"

[[acceptance]]
text = "Given a modal dialog or active command, when push runs, then R009 is reported and the file is untouched (adapters.md 4)"
bound = true
+++
