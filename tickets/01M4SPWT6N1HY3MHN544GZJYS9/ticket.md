+++
id = "01M4SPWT6N1HY3MHN544GZJYS9"
title = "cadetta ir, explain and units"
type = "story"
category = "todo"
priority = "medium"
points = 3
parent = "01M4SPWHHNRSGB9QZ04CMHWRRB"
reporter = "lognd"
created = "2026-10-13T04:03:56Z"
updated = "2026-10-13T04:03:56Z"
persona = "design engineer"
capability = "inspect IR, diagnostics and unit tables"
outcome_text = "I can debug without reading source"
labels = ["creates:crates/cadetta-cli/src/cmd/{ir,explain,units}.rs"]
scope = ["crates/cadetta-cli/src/cmd/{ir,explain,units}.rs"]

[[acceptance]]
text = "Given `cadetta explain Q007`, when run, then the catalog entry is printed"
bound = false

[[acceptance]]
text = "Given `cadetta ir`, when run, then cadetta.ir/1 JSON is printed"
bound = false

[[acceptance]]
text = "Given `cadetta units Length`, when run, then the Length units are listed"
bound = false
+++
