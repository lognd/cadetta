+++
id = "01M4VYZRQKXYE9WEFMDM3797GP"
title = "Evaluate indexed part-instance reads and re-expression (leg[1].top in origin)"
type = "bug"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-14T01:03:50Z"
updated = "2026-10-15T04:37:34Z"
scope = ["crates/cadetta-sema/**"]

[[acceptance]]
text = "Re-expression of an indexed part instance (leg[1].top in origin) evaluates at that instance placement"
bound = true

[[acceptance]]
text = "A member read through an indexed part slot (rail[0].length) resolves to the part type member"
bound = true

[[acceptance]]
text = "An out-of-range index or two instances of one slot in one in-expression is a diagnosed error"
bound = true
+++

found while working ~NYB5TXC: indexed part instance re-expression errors 'not evaluated yet'; blocks golden 06 array placement values.
