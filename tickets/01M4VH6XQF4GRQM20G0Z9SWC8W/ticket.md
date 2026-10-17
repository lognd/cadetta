+++
id = "01M4VH6XQF4GRQM20G0Z9SWC8W"
title = "SOLIDWORKS Windows bootstrap: .NET SDK, interop path, cadetta on Windows, API surface confirmed"
type = "story"
category = "done"
outcome = "done"
priority = "medium"
points = 3
parent = "01M4SV2TCZM23R46EM75RYSA3B"
reporter = "lognd"
created = "2026-10-13T21:03:04Z"
updated = "2026-10-17T06:15:07Z"
persona = "cadetta developer"
capability = "build and run the C# adapter against a real SOLIDWORKS"
outcome_text = "the C# tickets are built on confirmed APIs, not assumptions"
idempotency_key = "d15:sw-win-bootstrap"
labels = ["creates:adapters/solidworks/API-SURFACE.md", "creates:adapters/solidworks/DEVELOPING.md"]
scope = ["adapters/solidworks/API-SURFACE.md", "adapters/solidworks/DEVELOPING.md", "adapters/solidworks/**"]

[[acceptance]]
text = "Given the Windows machine, when bootstrapped, then the .NET SDK, SOLIDWORKS_API_DIR and a Windows cadetta build are in place and adapters/solidworks/DEVELOPING.md records the steps and versions"
bound = true

[[acceptance]]
text = "Given solidworks.md 4, when each listed API (equation manager globals and equations, IDimension and IDimensionTolerance, custom properties, coordinate system creation from numeric values, the coordinate system transform query, IComponent2 transform and reference property, ForceRebuild3) is exercised on the installed SOLIDWORKS version, then adapters/solidworks/API-SURFACE.md records the call used and the result, and any API that is missing becomes a spec ticket that removes or replaces the dependent feature"
bound = true
+++

Run on the owner's Windows machine with a licensed SOLIDWORKS. Do this before ~1EJXSV4, ~DKCF816 and the add-in tickets so they are built against confirmed calls.
