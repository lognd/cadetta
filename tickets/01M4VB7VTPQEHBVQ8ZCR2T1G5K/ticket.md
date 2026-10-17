+++
id = "01M4VB7VTPQEHBVQ8ZCR2T1G5K"
title = "SOLIDWORKS add-in: task pane, project discovery and CLI client"
type = "story"
category = "in-progress"
priority = "medium"
points = 8
parent = "01M4SV2TCZM23R46EM75RYSA3B"
reporter = "lognd"
created = "2026-10-13T19:18:44Z"
updated = "2026-10-17T06:37:38Z"
persona = "design engineer"
capability = "see a part's cadetta paths beside the model in SOLIDWORKS"
outcome_text = "linking starts from what the engineer selects"
idempotency_key = "d15:addin-pane"
labels = ["creates:adapters/solidworks/src/Cadetta.SolidWorks.AddIn/**", "creates:adapters/solidworks/tests/Cadetta.SolidWorks.Tests/AddIn/**"]
scope = ["design/model.grmb", "adapters/solidworks/src/Cadetta.SolidWorks.AddIn/**", "adapters/solidworks/tests/Cadetta.SolidWorks.Tests/AddIn/**"]

[[links]]
kind = "blocked-by"
target = "01M4SV2TGHW85Z0PZS8QHBFB4S"

[[links]]
kind = "blocked-by"
target = "01M4VB644Y6YPK6YTBP4T5YM3Y"

[[links]]
kind = "blocked-by"
target = "01M4VH6XQF4GRQM20G0Z9SWC8W"

[[acceptance]]
text = "Given an open document, when the pane loads, then it finds the nearest cadetta.toml, runs cadetta names solidworks --format json and lists the document component's paths with value, tolerance, citation and doc (solidworks.md 3)"
bound = false

[[acceptance]]
text = "Given a selected dimension, when the pane filters, then it shows paths of the dimension's kind, nearest value first (solidworks.md 2.1)"
bound = false

[[acceptance]]
text = "Given the add-in, when it runs the cadetta binary, then design/model.grmb records the exec flow from adapter_solidworks to cli and grimble check passes on the local and published frob"
bound = false

[[acceptance]]
text = 'Given the add-in, when logging, then it writes %LOCALAPPDATA%\cadetta\addin.log filtered by CADETTA_LOG (solidworks.md 1)'
bound = false
+++
