+++
id = "01M4X91P37CRMWF43MHK53RXEG"
title = "SOLIDWORKS transcript replay of emit, read, push, regen and contract rules"
type = "story"
category = "in-progress"
priority = "medium"
points = 3
parent = "01M4SV2TCZM23R46EM75RYSA3B"
reporter = "lognd"
created = "2026-10-14T13:18:53Z"
updated = "2026-10-17T05:21:32Z"
persona = "cadetta contributor"
capability = "test the adapter without SOLIDWORKS or Windows"
labels = ["creates:adapters/solidworks/tests/Cadetta.SolidWorks.Tests/Transcripts/**"]
scope = ["adapters/solidworks/tests/Cadetta.SolidWorks.Tests/Transcripts/**"]

[[links]]
kind = "blocked-by"
target = "01M4SV3AAC44Q1DCE44GG3XB6W"

[[acceptance]]
text = "Given recorded COM transcripts, when the fake replays them, then emit, read, push and regen tests pass on Linux CI (adapters.md 6)"
bound = false

[[acceptance]]
text = "Given the adapter contract harness, when run against the fake, then all five contract rules pass (adapters.md 4)"
bound = false
+++

Split from ~GG3XB6W: ISwSession has only ProductVersion, so emit/read/push/regen do not exist yet. Once they land, record transcripts and run them, and run the adapter contract harness against the fake.
