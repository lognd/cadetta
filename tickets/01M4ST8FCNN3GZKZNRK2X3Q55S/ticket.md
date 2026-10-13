+++
id = "01M4ST8FCNN3GZKZNRK2X3Q55S"
title = "cadetta compare"
type = "story"
category = "in-progress"
priority = "medium"
points = 5
parent = "01M4ST8EJWTGQM9CMVBJVBATJT"
reporter = "lognd"
created = "2026-10-13T05:02:44Z"
updated = "2026-10-13T09:44:46Z"
persona = "design engineer"
capability = "compare every implementer of an interface side by side"
idempotency_key = "replan13:cadetta compare"
labels = ["creates:crates/cadetta-cli/src/cmd/compare.rs"]
scope = ["crates/cadetta-cli/src/cmd/compare.rs", "crates/cadetta-cli/src/cmd/mod.rs", "crates/cadetta-cli/src/lib.rs"]

[[links]]
kind = "blocks"
target = "01M4ST8FEBCF9PC9SR0ZKTYT70"

[[acceptance]]
text = "Given `cadetta compare Caster`, when run, then every implementer is a column with interface members, bounds, commercial parameters, lifecycle and which checks hold for each in this design (obligations.md 5)"
bound = true

[[acceptance]]
text = "Given an interface used by several slots, when run, then the table covers every slot typed by it (obligations.md 5.1)"
bound = true
+++
