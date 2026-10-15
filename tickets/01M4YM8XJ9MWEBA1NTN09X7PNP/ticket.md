+++
id = "01M4YM8XJ9MWEBA1NTN09X7PNP"
title = "Carry waiver globs across renames and report eol W005"
type = "task"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-15T01:54:19Z"
updated = "2026-10-15T01:54:19Z"
scope = ["crates/cadetta-cli/src/**,crates/cadetta-lint/src/**"]
+++

Cut from ~203JFR7: waiver globs are not carried over renames, and no IR entity carries eol so check_w005 receives no eol dates.
