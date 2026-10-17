+++
id = "01M4SWMDPP54TE0XZGJRTKN07N"
title = "Snapshot and atomic restore"
type = "story"
category = "in-progress"
priority = "medium"
points = 5
parent = "01M4SV2TCZM23R46EM75RYSA3B"
reporter = "lognd"
created = "2026-10-13T05:44:12Z"
updated = "2026-10-17T05:15:44Z"
persona = "design engineer"
capability = "never leave a broken model"
idempotency_key = "replan14:sw-restore"
labels = ["creates:adapters/solidworks/src/Cadetta.SolidWorks.Core/Restore/**", "creates:adapters/solidworks/src/Cadetta.SolidWorks.Com/Restore/**", "creates:adapters/solidworks/tests/Cadetta.SolidWorks.Tests/Restore/**"]
scope = ["adapters/solidworks/src/Cadetta.SolidWorks.Core/Restore/**", "adapters/solidworks/src/Cadetta.SolidWorks.Com/Restore/**", "adapters/solidworks/tests/Cadetta.SolidWorks.Tests/Restore/**", "adapters/solidworks/src/Cadetta.SolidWorks.Tests/Restore/**", "adapters/solidworks/README.md", "changelog.d/01M4SWMDPP54TE0XZGJRTKN07N.*"]

[[links]]
kind = "blocked-by"
target = "01M4SWMC85Q2YCHQ3GS4MREKBP"

[[links]]
kind = "blocked-by"
target = "01M4SWMDFARQR4Q9HCBERB21PP"

[[acceptance]]
text = "Given a failed rebuild, when sync ends, then the pre-push file state is restored (adapters.md 2.3)"
bound = true
+++
