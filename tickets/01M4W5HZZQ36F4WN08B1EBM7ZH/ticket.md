+++
id = "01M4W5HZZQ36F4WN08B1EBM7ZH"
title = "Host Part argument and part arrays/count(at) for ScrewJoint"
type = "task"
category = "in-progress"
priority = "medium"
points = 8
reporter = "lognd"
created = "2026-10-14T02:58:39Z"
updated = "2026-10-15T13:42:37Z"
scope = ["crates/cadetta-sema/**", "tests/conformance/**", "docs/spec/language.md"]

[[acceptance]]
text = "Given host = <part> and at = Pattern2, when cadetta check runs, then joint-host-default-into, joint-count-*, joint-screw-cost evaluate"
bound = true
+++

found while working ~646D1T7: Part-typed instance args (nodef today), part screw[n] inside instances, count(j.screw), sum, count(at) with Pattern2.
