+++
id = "01M4ST8F3SDF95MVP07VXXNX0X"
title = "cadetta obligations"
type = "story"
category = "in-progress"
priority = "medium"
points = 3
parent = "01M4ST8EJWTGQM9CMVBJVBATJT"
reporter = "lognd"
created = "2026-10-13T05:02:43Z"
updated = "2026-10-13T08:57:15Z"
persona = "design engineer"
capability = "see the design to-do list of open and stale obligations"
idempotency_key = "replan13:cadetta obligations"
labels = ["creates:crates/cadetta-cli/src/cmd/obligations.rs"]
scope = ["crates/cadetta-cli/src/cmd/obligations.rs"]

[[acceptance]]
text = "Given a design, when `cadetta obligations [--stale] [--role R]` runs, then open and stale obligations are listed with their role (obligations.md 5)"
bound = false

[[acceptance]]
text = "Given `--format json`, when run, then the output is machine-readable (obligations.md 5)"
bound = false

[[acceptance]]
text = "Given --stale, when run, then only stale rows print and exit 1 follows required_for (config-cli.md 3)"
bound = false
+++
