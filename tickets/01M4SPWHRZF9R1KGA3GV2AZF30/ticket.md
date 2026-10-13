+++
id = "01M4SPWHRZF9R1KGA3GV2AZF30"
title = "CI pipeline and frob check gates"
type = "story"
category = "todo"
priority = "medium"
points = 3
parent = "01M4SPWFRR9DAEM60DX796GVPN"
reporter = "lognd"
created = "2026-10-13T04:03:47Z"
updated = "2026-10-13T04:04:03Z"
persona = "cadetta contributor"
capability = "run cargo fmt, clippy, test and frob check in CI"
outcome_text = "regressions and process violations are caught before merge"
labels = ["creates:.github/workflows/**"]
scope = [".github/workflows/**"]

[[links]]
kind = "blocks"
target = "01M4SPWSQQNCC23KA25TCFVQ98"

[[acceptance]]
text = "Given a pull request, when CI runs, then cargo fmt --check, clippy -D warnings, cargo test and frob check all execute"
bound = false

[[acceptance]]
text = "Given a lint violation, when CI runs, then the job fails"
bound = false
+++
