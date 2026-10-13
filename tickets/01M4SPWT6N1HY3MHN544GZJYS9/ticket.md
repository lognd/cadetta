+++
id = "01M4SPWT6N1HY3MHN544GZJYS9"
title = "cadetta ir, explain and units"
type = "story"
category = "in-progress"
priority = "medium"
points = 3
parent = "01M4SPWHHNRSGB9QZ04CMHWRRB"
reporter = "lognd"
created = "2026-10-13T04:03:56Z"
updated = "2026-10-13T09:03:41Z"
persona = "design engineer"
capability = "inspect IR, diagnostics and unit tables"
outcome_text = "I can debug without reading source"
labels = ["creates:crates/cadetta-cli/src/cmd/{ir,explain,units}.rs"]
scope = ["crates/cadetta-cli/src/cmd/{ir,explain,units}.rs", "crates/cadetta-cli/Cargo.toml", "crates/cadetta-units/src/units.rs", "crates/cadetta-units/src/kind.rs", "docs/schemas/explain.json", "docs/schemas/units.json", "docs/spec/config-cli.md", "Cargo.lock"]

[[acceptance]]
text = "Given `cadetta explain Q007`, when run, then the catalog entry is printed"
bound = false

[[acceptance]]
text = "Given `cadetta ir`, when run, then cadetta.ir/1 JSON is printed"
bound = false

[[acceptance]]
text = "Given `cadetta units Length`, when run, then the Length units are listed"
bound = false

[[acceptance]]
text = "Given `cadetta ir --observed freecad`, when run, then observed IR as cadetta.ir/1 JSON is printed (config-cli.md 2)"
bound = false

[[acceptance]]
text = "Given ir, explain and units, when run, then outputs validate against ir.json, explain.json and units.json (config-cli.md 3)"
bound = false
+++
