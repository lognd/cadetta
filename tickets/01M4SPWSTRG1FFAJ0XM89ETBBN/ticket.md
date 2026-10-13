+++
id = "01M4SPWSTRG1FFAJ0XM89ETBBN"
title = "cadetta.toml configuration model"
type = "story"
category = "todo"
priority = "medium"
points = 3
parent = "01M4SPWHHNRSGB9QZ04CMHWRRB"
reporter = "lognd"
created = "2026-10-13T04:03:55Z"
updated = "2026-10-13T04:03:55Z"
persona = "design engineer"
capability = "configure project, units, emit, lint and adapters in cadetta.toml"
outcome_text = "behaviour is declared and versioned with the repo"
labels = ["creates:crates/cadetta-cli/src/config.rs"]
scope = ["crates/cadetta-cli/src/config.rs"]

[[acceptance]]
text = "Given the config-cli.md sample, when loaded, then all sections parse"
bound = false

[[acceptance]]
text = "Given an unknown key, when loaded, then a diagnostic names it"
bound = false
+++
