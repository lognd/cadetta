+++
id = "01M4YP8J587Y5AFZ4E4HZPVB08"
title = "solve --variants does not validate declared outputs (S005)"
type = "bug"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-15T02:29:04Z"
updated = "2026-10-15T02:29:51Z"
scope = ["crates/cadetta-cli/src/cmd/solve.rs", "crates/cadetta-solve/src/variants.rs", "docs/spec/solvers.md", "changelog.d/01M4YP8J587Y5AFZ4E4HZPVB08.fixed.md"]

[[acceptance]]
text = "solve --variants rejects missing or wrong-kind variant outputs with S005 and stores nothing"
bound = false
+++

found while working ~QDVP8JQ: check_outputs guards only the main solve path; variants::store accepts missing or wrong-kind outputs.
