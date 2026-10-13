+++
id = "01M4SV2TR8ENTA87J1DDKCF816"
title = "SOLIDWORKS emit and push: globals, dimension values and ref mode"
type = "story"
category = "todo"
priority = "medium"
points = 8
parent = "01M4SV2TCZM23R46EM75RYSA3B"
reporter = "lognd"
created = "2026-10-13T05:17:07Z"
updated = "2026-10-13T19:18:41Z"
persona = "design engineer"
capability = "write cadetta values into SOLIDWORKS and keep dimensions linked"
labels = ["creates:adapters/solidworks/src/Cadetta.SolidWorks.Core/Push/**", "creates:adapters/solidworks/src/Cadetta.SolidWorks.Com/Push/**", "creates:adapters/solidworks/tests/Cadetta.SolidWorks.Tests/Push/**"]
scope = ["adapters/solidworks/src/Cadetta.SolidWorks.Core/Push/**", "adapters/solidworks/src/Cadetta.SolidWorks.Com/Push/**", "adapters/solidworks/tests/Cadetta.SolidWorks.Tests/Push/**"]

[[links]]
kind = "blocked-by"
target = "01M4SPWRYRBTGD67WTG7D60E59"

[[links]]
kind = "blocked-by"
target = "01M4SPWS23PN7M3TKKW8SA8QRK"

[[links]]
kind = "blocked-by"
target = "01M4SV2TGHW85Z0PZS8QHBFB4S"

[[links]]
kind = "blocked-by"
target = "01M4SV3AAC44Q1DCE44GG3XB6W"

[[acceptance]]
text = "Given a path bound to several locators, when emitted, then each global, dimension and property is written (adapters.md 3)"
bound = false

[[acceptance]]
text = "Given a ref-mode dimension, when emitted, then it links to its ref_to global, else the first global bound to the path, else the path's own mangled global (adapters.md 3)"
bound = false

[[acceptance]]
text = "Given a value not representable at precision, when emitted, then Q004 is returned (adapters.md 4 rule 3)"
bound = false

[[acceptance]]
text = "Given a changed digest, when pushed, then only that locator is updated (adapters.md 5)"
bound = false

[[acceptance]]
text = "Given save and guard settings, when pushing, then per-locator status is returned and the save policy honoured (adapters.md 2.2, 5)"
bound = false
+++
