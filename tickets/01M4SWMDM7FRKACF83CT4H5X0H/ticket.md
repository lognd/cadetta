+++
id = "01M4SWMDM7FRKACF83CT4H5X0H"
title = "Save after sync and file digest"
type = "story"
category = "in-progress"
priority = "medium"
points = 3
parent = "01M4SV2TCZM23R46EM75RYSA3B"
reporter = "lognd"
created = "2026-10-13T05:44:12Z"
updated = "2026-10-15T06:01:45Z"
persona = "design engineer"
capability = "persist pushed values"
idempotency_key = "replan14:sw-save"
labels = ["creates:adapters/solidworks/src/Cadetta.SolidWorks.Core/Save/**", "creates:adapters/solidworks/src/Cadetta.SolidWorks.Com/Save/**", "creates:adapters/solidworks/tests/Cadetta.SolidWorks.Tests/Save/**"]
scope = ["adapters/solidworks/src/Cadetta.SolidWorks.Core/Save/**", "adapters/solidworks/src/Cadetta.SolidWorks.Com/Save/**", "adapters/solidworks/tests/Cadetta.SolidWorks.Tests/Save/**", "adapters/solidworks/src/Cadetta.SolidWorks.Tests/Save/**", "docs/spec/solidworks.md", "changelog.d/01M4SWMDM7FRKACF83CT4H5X0H.*"]

[[links]]
kind = "blocked-by"
target = "01M4SWMDFARQR4Q9HCBERB21PP"

[[acceptance]]
text = "Given save = after-sync, when sync succeeds, then files are saved and file digests recorded; save = never leaves them dirty (adapters.md 2.2; config-cli.md 2)"
bound = false
+++
