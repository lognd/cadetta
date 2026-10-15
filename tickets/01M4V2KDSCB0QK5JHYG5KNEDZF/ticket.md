+++
id = "01M4V2KDSCB0QK5JHYG5KNEDZF"
title = "Lint gaps found probing lints: D001 severity, D002 fit members, M003, C004, geometry pass unwired"
type = "bug"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T16:47:45Z"
updated = "2026-10-15T04:14:11Z"
scope = ["crates/cadetta-lint/**"]

[[acceptance]]
text = "D001 is info below d001_min_digits"
bound = false

[[acceptance]]
text = "D002 fires for fit clearance accessors"
bound = false

[[acceptance]]
text = "Geometry G002-G007 reach check"
bound = false
+++

Probing for ~T71T26K: D001 warns for low-digit equal groups (spec: info below d001_min_digits); D002 unreachable (fit member chains give EVAL); M003 silent for path/path fits; C004 silent; G002-G007 not wired into check (cadetta-geometry unused). Fixtures for these were not committed because they cannot pass yet.
