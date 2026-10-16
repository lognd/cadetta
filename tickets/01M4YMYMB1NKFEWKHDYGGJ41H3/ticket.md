+++
id = "01M4YMYMB1NKFEWKHDYGGJ41H3"
title = "Lint E-family resolution errors through e::lift_resolution in check"
type = "task"
category = "in-progress"
priority = "medium"
points = 2
reporter = "lognd"
created = "2026-10-15T02:06:10Z"
updated = "2026-10-16T04:29:52Z"
scope = ["crates/cadetta-cli/src/pipeline.rs", "docs/spec/lints.md"]

[[acceptance]]
text = "Resolver E-family errors are lifted through e::lift_resolution (notes for E003/E004 related spans), message text unchanged"
bound = false
+++

found while working ~4H7PTZX: resolve errors surface as fixed errors via located(); routing through cadetta_lint e::lift_resolution changes message text.
