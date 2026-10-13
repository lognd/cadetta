+++
id = "01M4SV3AAC44Q1DCE44GG3XB6W"
title = "Recorded-COM-transcript fake for SOLIDWORKS CI tests"
type = "story"
category = "todo"
priority = "medium"
points = 5
parent = "01M4SV2TCZM23R46EM75RYSA3B"
reporter = "lognd"
created = "2026-10-13T05:17:23Z"
updated = "2026-10-13T05:17:23Z"
persona = "cadetta contributor"
capability = "test the adapter without SOLIDWORKS or Windows"
labels = ["creates:adapters/solidworks/tests/**", "creates:.github/workflows/solidworks.yml"]
scope = ["adapters/solidworks/tests/**", ".github/workflows/solidworks.yml"]

[[links]]
kind = "blocked-by"
target = "01M4ST7B6MNGRQ6967DAWQA5YK"

[[links]]
kind = "blocked-by"
target = "01M4SV2TGHW85Z0PZS8QHBFB4S"

[[acceptance]]
text = "Given recorded COM transcripts, when the fake replays them, then emit, read, push and regen tests pass on Linux CI (adapters.md 6)"
bound = false

[[acceptance]]
text = "Given the adapter contract harness, when run against the fake, then all five contract rules pass (adapters.md 4)"
bound = false
+++
