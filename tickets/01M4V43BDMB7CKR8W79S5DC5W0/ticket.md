+++
id = "01M4V43BDMB7CKR8W79S5DC5W0"
title = "Geometry datum declaration checks not enforced"
type = "bug"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T17:13:56Z"
updated = "2026-10-13T17:13:56Z"
scope = ["crates/cadetta-sema/**"]
+++

found while working ~ZFM3633. Fixtures datum-bound-*, datum-built-from-earlier, datum-flip-z, datum-ots-needs-citation, datum-unknown-of-placed-part, datum-zero-count-pattern and i001-interface-datum-omitted expect C001/C002 on datum bounds, E002/E006 decisions (geometry.md conformance decisions), S001 and I001; the binary emits none of them.
