+++
id = "01M4SPWSTRG1FFAJ0XM89ETBBN"
title = "cadetta.toml configuration model"
type = "story"
category = "in-progress"
priority = "medium"
points = 5
parent = "01M4SPWHHNRSGB9QZ04CMHWRRB"
reporter = "lognd"
created = "2026-10-13T04:03:55Z"
updated = "2026-10-13T06:23:20Z"
persona = "design engineer"
capability = "configure project, units, emit, lint and adapters in cadetta.toml"
outcome_text = "behaviour is declared and versioned with the repo"
labels = ["creates:crates/cadetta-cli/src/config.rs"]
scope = ["crates/cadetta-cli/src/config.rs", "Cargo.toml", "crates/cadetta-cli/Cargo.toml", "crates/cadetta-cli/src/lib.rs", "docs/spec/config-cli.md", "Cargo.lock"]

[[links]]
kind = "blocks"
target = "01M4SPWSXBTNXW3CPZ6FVHT5ZT"

[[acceptance]]
text = "Given the config-cli.md sample, when loaded, then all sections parse"
bound = true

[[acceptance]]
text = "Given an unknown key, when loaded, then a diagnostic names it"
bound = true

[[acceptance]]
text = "Given `[lint] severity`, `d001_min_digits`, `common_values` and `precision`, when loaded, then each is typed and validated (config-cli.md 1)"
bound = true

[[acceptance]]
text = "Given `[obligations.roles.X]` with covers and required_for, when loaded, then roles are modelled (obligations.md 4)"
bound = true

[[acceptance]]
text = "Given `[solvers.NAME]` and `[[adapter]]` tables, when loaded, then command, transport, native_unit, version and files parse (config-cli.md 1)"
bound = true

[[acceptance]]
text = "Given `[emit] point` and `[units] display`, when loaded, then they feed emit policy and display (quantities.md 5.6, 6)"
bound = true

[[acceptance]]
text = "Given cadetta.toml, when parsed, then [defaults], [units], [emit], [lint], [obligations], [solvers] and [[adapter]] keys have the types and defaults of config-cli.md 2"
bound = true

[[acceptance]]
text = "Given [package] name, version, src and as_of, [dependencies] path and git entries with version and trust, and a [workspace] members table, when loaded, then each has the type and default of config-cli.md 2 and packages.md 1 (the old [project] namespace, sources and roots keys are rejected)"
bound = true
+++
