+++
id = "01M4ST8F5G8E0FMC3KZJG8A95T"
title = "cadetta why"
type = "story"
category = "in-progress"
priority = "medium"
points = 5
parent = "01M4ST8EJWTGQM9CMVBJVBATJT"
reporter = "lognd"
created = "2026-10-13T05:02:43Z"
updated = "2026-10-13T09:00:32Z"
persona = "design engineer"
capability = "trace a value back to its citations, tables, solvers and requirements"
idempotency_key = "replan13:cadetta why"
labels = ["creates:crates/cadetta-cli/src/cmd/why.rs"]
scope = ["crates/cadetta-cli/src/cmd/why.rs", "crates/cadetta-cli/src/cmd/mod.rs", "crates/cadetta-cli/src/lib.rs", "docs/schemas/why.json", "docs/spec/obligations.md"]

[[acceptance]]
text = "Given a path, when `cadetta why PATH` runs, then the derivation chain upstream to citations, tables, solvers and requirements is printed (obligations.md 5)"
bound = false

[[acceptance]]
text = "Given `--format json`, when run, then the chain is machine-readable (obligations.md 5)"
bound = false

[[acceptance]]
text = "Given why PATH, when run, then output validates against why.json (config-cli.md 3)"
bound = false
+++
