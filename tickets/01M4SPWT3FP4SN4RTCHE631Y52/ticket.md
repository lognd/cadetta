+++
id = "01M4SPWT3FP4SN4RTCHE631Y52"
title = "cadetta check"
type = "story"
category = "done"
outcome = "done"
priority = "medium"
points = 5
parent = "01M4SPWHHNRSGB9QZ04CMHWRRB"
reporter = "lognd"
created = "2026-10-13T04:03:56Z"
updated = "2026-10-13T09:41:52Z"
persona = "design engineer"
capability = "run `cadetta check`"
outcome_text = "I get lint diagnostics and CI gating in one command"
labels = ["creates:crates/cadetta-cli/src/cmd/check.rs"]
scope = ["crates/cadetta-cli/src/cmd/check.rs", "crates/cadetta-cli/src/pipeline.rs", "crates/cadetta-cli/src/cmd/mod.rs", "crates/cadetta-cli/src/lib.rs", "design/model.grmb"]

[[links]]
kind = "blocks"
target = "01M4SPWTDD069AXXQBXD4T6ATZ"

[[links]]
kind = "blocks"
target = "01M4STCFQMGTXQAYPANNCHWEN4"

[[acceptance]]
text = "Given an Axle with Q001, when checked, then exit code is 1 and the diagnostic has code, path and span"
bound = true

[[acceptance]]
text = "Given adapters with read, when checked, then drift lints also run"
bound = true

[[acceptance]]
text = "Given `cadetta check --release`, when run, then stale or open obligations whose role is required_for release also fail (obligations.md 4)"
bound = true

[[acceptance]]
text = 'Given stale obligations with required_for = "check", when `cadetta check` runs, then it exits 1 (obligations.md 4)'
bound = true

[[acceptance]]
text = "Given check flags --release --as-of --watch --module PATH GLOB, when run, then exit 1 follows fail_on (config-cli.md 3)"
bound = true
+++
