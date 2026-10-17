+++
id = "01M555V54C2E8R8R5Y1E53Y69V"
title = "Ansys adapter: choose the product and write the spec"
type = "story"
category = "in-progress"
priority = "low"
points = 3
reporter = "lognd"
created = "2026-10-17T14:56:49Z"
updated = "2026-10-17T15:29:14Z"
scope = ["docs/spec/ansys.md", "docs/spec/adapters.md"]

[[acceptance]]
text = "docs/spec/ansys.md picks the Ansys product (Workbench parameters via PyAnsys is the recommended default; Mechanical and SpaceClaim are alternatives) and records the rationale"
bound = false

[[acceptance]]
text = "The spec maps bindings to the product parameters, units and the update/solve cycle, and states the licensing and test-host requirement (no Ansys install was found on nova on 2026-10-17)"
bound = false

[[acceptance]]
text = "The adapters.md target matrix gains an Ansys row"
bound = false
+++
