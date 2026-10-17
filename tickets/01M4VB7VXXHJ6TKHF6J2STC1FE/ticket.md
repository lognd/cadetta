+++
id = "01M4VB7VXXHJ6TKHF6J2STC1FE"
title = "SOLIDWORKS add-in commands: link dimension, assign part and array, create datum frames, sync"
type = "story"
category = "done"
outcome = "done"
priority = "medium"
points = 8
parent = "01M4SV2TCZM23R46EM75RYSA3B"
reporter = "lognd"
created = "2026-10-13T19:18:44Z"
updated = "2026-10-17T07:12:29Z"
persona = "design engineer"
capability = "link CAD items to cadetta paths by selection"
outcome_text = "no bindings file entry restates a link SOLIDWORKS already holds"
idempotency_key = "d15:addin-commands"
labels = ["creates:adapters/solidworks/src/Cadetta.SolidWorks.AddIn/Commands/**", "creates:adapters/solidworks/src/Cadetta.SolidWorks.Core/Commands/**", "creates:adapters/solidworks/tests/Cadetta.SolidWorks.Tests/Commands/**"]
scope = ["adapters/solidworks/src/Cadetta.SolidWorks.AddIn/Commands/**", "adapters/solidworks/src/Cadetta.SolidWorks.Core/Commands/**", "adapters/solidworks/tests/Cadetta.SolidWorks.Tests/Commands/**", "adapters/solidworks/src/Cadetta.SolidWorks.AddIn/README.md"]

[[links]]
kind = "blocked-by"
target = "01M4SV2TR8ENTA87J1DDKCF816"

[[links]]
kind = "blocked-by"
target = "01M4VB7VTPQEHBVQ8ZCR2T1G5K"

[[acceptance]]
text = "Given Link dimension, when run, then the dimension's equation references the path's created global (created if missing) and its tolerance follows adapters.md 3.6; nothing is written to the bindings file (solidworks.md 3)"
bound = true

[[acceptance]]
text = "Given Bind as value, Assign part and Assign array, when run, then cadetta bind is called, the reference property is set, and array indices are assigned by unique nearest transform or refused (solidworks.md 2.2, 3)"
bound = true

[[acceptance]]
text = "Given Create datum frames, Show path, Sync and Dry run, when run, then they behave as solidworks.md 3 states and refuse a document a running sync holds"
bound = true
+++
