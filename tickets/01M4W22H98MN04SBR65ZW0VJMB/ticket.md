+++
id = "01M4W22H98MN04SBR65ZW0VJMB"
title = "Write the transform composition rule into adapters.md 3.4"
type = "task"
category = "done"
outcome = "done"
priority = "low"
points = 1
reporter = "lognd"
created = "2026-10-14T01:57:46Z"
updated = "2026-10-16T00:38:26Z"
scope = ["docs/spec/adapters.md"]

[[acceptance]]
text = "adapters.md 3.4 states the transform composition rule"
bound = true
+++

found while working 645FHJZ: the composition rule (datum ops left to right, anchor node resolution for at on another part) is documented in the geometry_bind.rs module docs; docs/spec/** was leased by ~MCJWA2D. Patch to apply: bullet 'Composition.' before 'A placement is verify-only' in 3.4.
