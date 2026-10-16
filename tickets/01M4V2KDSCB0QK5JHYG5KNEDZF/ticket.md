+++
id = "01M4V2KDSCB0QK5JHYG5KNEDZF"
title = "Lint gaps found probing lints: D001 severity, D002 fit members, M003, C004, geometry pass unwired"
type = "bug"
category = "in-progress"
priority = "medium"
points = 3
reporter = "lognd"
created = "2026-10-13T16:47:45Z"
updated = "2026-10-16T04:56:54Z"
scope = ["crates/cadetta-lint/**"]

[[acceptance]]
text = "D001 is info below d001_min_digits"
bound = true

[[acceptance]]
text = "D002 fires for fit clearance accessors"
bound = true

[[acceptance]]
text = "Geometry G002-G007 reach check"
bound = true

[[acceptance]]
text = "D002 sees fit hole/shaft accessors and M003 fires for path/path fit sides (blocked on ~3YZ3NSG IR decision)"
bound = true
+++

Probing for ~T71T26K: D001 warns for low-digit equal groups (spec: info below d001_min_digits); D002 unreachable (fit member chains give EVAL); M003 silent for path/path fits; C004 silent; G002-G007 not wired into check (cadetta-geometry unused). Fixtures for these were not committed because they cannot pass yet.
