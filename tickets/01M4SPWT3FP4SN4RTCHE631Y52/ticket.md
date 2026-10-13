+++
id = "01M4SPWT3FP4SN4RTCHE631Y52"
title = "cadetta check"
type = "story"
category = "todo"
priority = "medium"
points = 5
parent = "01M4SPWHHNRSGB9QZ04CMHWRRB"
reporter = "lognd"
created = "2026-10-13T04:03:56Z"
updated = "2026-10-13T04:04:05Z"
persona = "design engineer"
capability = "run `cadetta check`"
outcome_text = "I get lint diagnostics and CI gating in one command"
labels = ["creates:crates/cadetta-cli/src/cmd/check.rs"]
scope = ["crates/cadetta-cli/src/cmd/check.rs"]

[[links]]
kind = "blocks"
target = "01M4SPWTDD069AXXQBXD4T6ATZ"

[[acceptance]]
text = "Given an Axle with Q001, when checked, then exit code is 1 and the diagnostic has code, path and span"
bound = false

[[acceptance]]
text = "Given adapters with read, when checked, then drift lints also run"
bound = false
+++
