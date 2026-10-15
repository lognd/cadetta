+++
id = "01M4V3F2DQTCGEAPVNHQDVP8JQ"
title = "solve accepts missing and wrong-kind solver outputs"
type = "bug"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T17:02:51Z"
updated = "2026-10-15T02:28:05Z"
scope = ["crates/cadetta-cli/src/**", "tests/conformance/known-gaps.toml", "docs/**", "changelog.d/**"]

[[acceptance]]
text = "solvers.md 3: solve rejects a response with a missing or wrong-kind output as S005 and stores nothing (fixtures solvers/s005-output-missing, s005-output-wrong-kind; found while working ~KBYTYF0)"
bound = false
+++
