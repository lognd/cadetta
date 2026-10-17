+++
id = "01M4SWME4W423G9HT6E2VJJ6M5"
title = "Read cache keyed by file digest"
type = "story"
category = "in-progress"
priority = "medium"
points = 2
parent = "01M4SV2TCZM23R46EM75RYSA3B"
reporter = "lognd"
created = "2026-10-13T05:44:13Z"
updated = "2026-10-17T05:18:46Z"
persona = "design engineer"
capability = "make check fast"
idempotency_key = "replan14:sw-cache"
labels = ["creates:adapters/solidworks/src/Cadetta.SolidWorks.Core/Cache/**", "creates:adapters/solidworks/src/Cadetta.SolidWorks.Com/Cache/**", "creates:adapters/solidworks/tests/Cadetta.SolidWorks.Tests/Cache/**"]
scope = ["adapters/solidworks/src/Cadetta.SolidWorks.Core/Cache/**", "adapters/solidworks/src/Cadetta.SolidWorks.Com/Cache/**", "adapters/solidworks/tests/Cadetta.SolidWorks.Tests/Cache/**", "adapters/solidworks/src/Cadetta.SolidWorks.Tests/Cache/**"]

[[links]]
kind = "blocked-by"
target = "01M4SWMDM7FRKACF83CT4H5X0H"

[[acceptance]]
text = "Given an unchanged file digest, when read runs again, then the cached observed IR is returned without opening the file (adapters.md 2.2)"
bound = false
+++
