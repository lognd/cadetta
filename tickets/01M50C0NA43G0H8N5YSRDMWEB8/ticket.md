+++
id = "01M50C0NA43G0H8N5YSRDMWEB8"
title = "Units: disc (u,v) source terms in Affine with exact and conservative bounds"
type = "task"
category = "in-progress"
priority = "medium"
points = 3
reporter = "lognd"
created = "2026-10-15T18:08:28Z"
updated = "2026-10-16T04:04:48Z"
idempotency_key = "v2k95h5-slice-units"
scope = ["crates/cadetta-units/src/**"]

[[acceptance]]
text = "Affine carries paired u/v disc terms with exact bounds, and existing L1-box callers are unchanged."
bound = false
+++

Add a disc source variable (u^2+v^2<=1) to Affine; radius/bounds/combine/scale/mul/div and the conservative flag must handle it. Split from ~V2K95H5 (too large for one slice).
