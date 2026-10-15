+++
id = "01M4VYZS8VGT1RASS407Y8K809"
title = "Evaluate datums and placements of parameterized parts"
type = "bug"
category = "in-progress"
priority = "medium"
points = 3
reporter = "lognd"
created = "2026-10-14T01:03:51Z"
updated = "2026-10-15T08:49:34Z"
scope = ["crates/cadetta-sema/**", "docs/spec/ir.md", "docs/spec/language.md"]

[[acceptance]]
text = "Datums of a parameterized instance are evaluated per instance (given argument and default) and lower as datum Members of the instantiating component"
bound = true

[[acceptance]]
text = "Placements inside a parameterized assembly instance are evaluated per instance and lower as place Members"
bound = true
+++

found while working ~NYB5TXC: geom_components excludes parameterized components; they lower structurally with valueless geometry members.
