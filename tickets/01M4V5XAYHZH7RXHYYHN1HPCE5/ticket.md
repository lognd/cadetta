+++
id = "01M4V5XAYHZH7RXHYYHN1HPCE5"
title = "Conformance README: define kind=binding entry=python fixture form"
type = "task"
category = "in-progress"
priority = "medium"
points = 1
reporter = "lognd"
created = "2026-10-13T17:45:36Z"
updated = "2026-10-15T06:10:36Z"
scope = ["tests/conformance/README.md"]

[[acceptance]]
text = "tests/conformance/README.md documents the binding fixture keys (api/args/files/context) and result/exit vocabulary, linked from the kind=binding row"
bound = false
+++

found while working ~VNWRJHC. README defines kind=binding but not the call-description form (api, args, files) or result vocabulary; documented only in tests/conformance/bindings/README.md. Promote it to the main README when ~6ZMPC6J lands.
