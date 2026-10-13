+++
id = "01M4SWFX6PAJ8AK9NND3070SN1"
title = "Interface datums and datum bounds"
type = "story"
category = "todo"
priority = "medium"
points = 3
parent = "01M4SWEQMQ8DQMN8B8GWVR4GDH"
reporter = "lognd"
created = "2026-10-13T05:41:44Z"
updated = "2026-10-13T05:41:44Z"
persona = "design engineer"
capability = "swap parts without losing placements"
idempotency_key = "replan14:geo-ifdatum"
labels = ["creates:crates/cadetta-geometry/src/interface_datum.rs"]
scope = ["crates/cadetta-geometry/src/interface_datum.rs"]

[[links]]
kind = "blocked-by"
target = "01M4ST8E8AQ98D0DFQ0DT50R91"

[[links]]
kind = "blocked-by"
target = "01M4SWFX374CNSPDDD289Z9BCS"

[[acceptance]]
text = "Given an interface declaring datum mount, when an implementer omits it, then implementation checking fails (geometry.md 2)"
bound = false

[[acceptance]]
text = "Given a bound shaft.z == mount.z on interface datums, when an implementer has the opposite convention, then C001 is raised against that implementer (geometry.md 2)"
bound = false

[[acceptance]]
text = "Given a swap to an alternate, when a placement uses mount, then the alternate's own mount is used (geometry.md 2)"
bound = false
+++
