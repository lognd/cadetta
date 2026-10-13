+++
id = "01M4ST8F7B9VYMFSCJD8MVBJNS"
title = "cadetta impact"
type = "story"
category = "todo"
priority = "medium"
points = 5
parent = "01M4ST8EJWTGQM9CMVBJVBATJT"
reporter = "lognd"
created = "2026-10-13T05:02:43Z"
updated = "2026-10-13T05:44:40Z"
persona = "design engineer"
capability = "see everything that goes stale or flips if a value changes"
idempotency_key = "replan13:cadetta impact"
labels = ["creates:crates/cadetta-cli/src/cmd/impact.rs"]
scope = ["crates/cadetta-cli/src/cmd/impact.rs"]

[[links]]
kind = "blocks"
target = "01M4ST8F91FHYFC01TN6018BAM"

[[acceptance]]
text = "Given a path, when `cadetta impact PATH` runs, then every downstream node that goes stale or changes verdict is listed (obligations.md 5)"
bound = false

[[acceptance]]
text = "Given a path with no dependents, when run, then the output says so (obligations.md 5)"
bound = false

[[acceptance]]
text = "Given impact PATH, when run, then output validates against impact.json (config-cli.md 3)"
bound = false
+++
