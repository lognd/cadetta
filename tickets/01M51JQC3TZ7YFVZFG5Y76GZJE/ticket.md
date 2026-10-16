+++
id = "01M51JQC3TZ7YFVZFG5Y76GZJE"
title = "docs/spec/ir.md: document fit accessor members (<fit>.hole/.shaft/.clearance)"
type = "docs"
category = "in-progress"
priority = "medium"
points = 1
reporter = "lognd"
created = "2026-10-16T05:24:59Z"
updated = "2026-10-16T07:23:31Z"
scope = ["docs/spec/ir.md"]

[[acceptance]]
text = "ir.md documents fit accessor members as derived Let members"
bound = true
+++

found while working ~93GZBM2 (ir.md leased by ~JCJY1Z8). After 'Fit operands' add: accessor reads are also emitted as derived Let members of the owning component (provenance derived, analysis, deps [fit path]); no schema change; digest keys .clearance on its member node, .hole/.shaft keep fit-side nodes until the add_fit follow-up lands.
