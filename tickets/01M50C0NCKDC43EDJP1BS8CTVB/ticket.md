+++
id = "01M50C0NCKDC43EDJP1BS8CTVB"
title = "Geometry: disc arithmetic for point.x/.y projection, aligned and distance"
type = "task"
category = "in-progress"
priority = "medium"
points = 3
reporter = "lognd"
created = "2026-10-15T18:08:29Z"
updated = "2026-10-16T04:14:15Z"
idempotency_key = "v2k95h5-slice-geometry"
scope = ["crates/cadetta-geometry/src/**", "crates/cadetta-units/src/tolerance.rs"]

[[links]]
kind = "blocked-by"
target = "01M50C0NA43G0H8N5YSRDMWEB8"

[[acceptance]]
text = "Projection, aligned and distance over position discs meet the exact and conservative bounds in the spec."
bound = false
+++

Exact .x bound [-d/2,d/2]; Euclidean norm for mixed projection after non-90deg rotation; aligned/distance exact for coplanar discs, conservative for tilted. Blocked by the units slice. Split from ~V2K95H5 (too large for one slice).
