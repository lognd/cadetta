+++
id = "01M4SWMDS8MDHBS4T9JX1R3K6N"
title = "Push guard R010 and R006"
type = "story"
category = "in-progress"
priority = "medium"
points = 5
parent = "01M4SV2TCZM23R46EM75RYSA3B"
reporter = "lognd"
created = "2026-10-13T05:44:12Z"
updated = "2026-10-17T06:50:21Z"
persona = "design engineer"
capability = "see what a push would overwrite"
idempotency_key = "replan14:sw-guard"
labels = ["creates:adapters/solidworks/src/Cadetta.SolidWorks.Core/Guard/**", "creates:adapters/solidworks/src/Cadetta.SolidWorks.Com/Guard/**", "creates:adapters/solidworks/tests/Cadetta.SolidWorks.Tests/Guard/**"]
scope = ["adapters/solidworks/src/Cadetta.SolidWorks.Core/Guard/**", "adapters/solidworks/src/Cadetta.SolidWorks.Com/Guard/**", "adapters/solidworks/tests/Cadetta.SolidWorks.Tests/Guard/**", "adapters/solidworks/src/Cadetta.SolidWorks.Core/Push/IPushDocument.cs"]

[[links]]
kind = "blocked-by"
target = "01M4SV2TR8ENTA87J1DDKCF816"

[[links]]
kind = "blocked-by"
target = "01M4SWMDFARQR4Q9HCBERB21PP"

[[acceptance]]
text = "Given a read-only file, when pushing, then R010 is reported; given an existing equation, then R006 lists it before replacing (adapters.md 4)"
bound = false
+++
