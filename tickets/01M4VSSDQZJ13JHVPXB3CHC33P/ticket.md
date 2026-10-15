+++
id = "01M4VSSDQZJ13JHVPXB3CHC33P"
title = "Counterbore data for ButtonHead and FlatHead (ISO 7380, ISO 10642)"
type = "story"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T23:32:59Z"
updated = "2026-10-15T04:30:52Z"
scope = ["std/src/fasteners/**"]

[[acceptance]]
text = "Given head = ButtonHead or FlatHead and counterbore = true, when evaluated, then the pocket comes from Iso7380Head or Iso10642Head (stdlib.md 3)"
bound = true
+++

found while working ~XCK8EGP: head argument accepts ButtonHead/FlatHead but only Iso4762Head has counterbore data; counterbore = true with them is undiagnosed.
