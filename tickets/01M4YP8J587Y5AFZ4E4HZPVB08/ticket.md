+++
id = "01M4YP8J587Y5AFZ4E4HZPVB08"
title = "solve --variants does not validate declared outputs (S005)"
type = "bug"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-15T02:29:04Z"
updated = "2026-10-15T02:29:04Z"
scope = ["crates/cadetta-cli/src/cmd/solve.rs", "crates/cadetta-solve/src/variants.rs"]
+++

found while working ~QDVP8JQ: check_outputs guards only the main solve path; variants::store accepts missing or wrong-kind outputs.
