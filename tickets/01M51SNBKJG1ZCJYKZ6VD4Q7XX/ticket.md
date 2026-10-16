+++
id = "01M51SNBKJG1ZCJYKZ6VD4Q7XX"
title = "conformance: executors and fixtures for the update and obligations entries"
type = "task"
category = "in-progress"
priority = "medium"
points = 2
reporter = "lognd"
created = "2026-10-16T07:26:13Z"
updated = "2026-10-16T10:46:35Z"
scope = ["crates/cadetta-cli/tests/conformance/**", "tests/conformance/packages/**", "tests/conformance/known-gaps.toml"]

[[acceptance]]
text = "The conformance runner has update and obligations executors, and the update-*/trust-* package fixtures pass (E012/E013 exit mismatch tracked in known-gaps)."
bound = false
+++

Follow-up of ~XXKBNB8: implement the update and obligations entries specified in tests/conformance/README.md and add fixtures for update (drop records, NAME not in closure exit 2, E012/E013 not written) and obligations (attested upstream row, trusted role discharge).
