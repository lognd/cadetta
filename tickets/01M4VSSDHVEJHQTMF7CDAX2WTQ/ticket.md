+++
id = "01M4VSSDHVEJHQTMF7CDAX2WTQ"
title = "E006 for count with at/onto and one-joint-per-pattern part arrays"
type = "story"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T23:32:59Z"
updated = "2026-10-15T04:33:04Z"
scope = ["crates/cadetta-sema/**"]

[[acceptance]]
text = "count together with at/onto is E006 (ScrewJoint forbids count when at is given)"
bound = true

[[acceptance]]
text = "a part array of a component taking a Pattern argument is E006 (one joint per pattern)"
bound = true
+++

found while working ~XCK8EGP: stdlib.md 3 rules e006-count-with-pattern and e006-joint-per-pattern.
