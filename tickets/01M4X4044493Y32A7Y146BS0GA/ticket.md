+++
id = "01M4X4044493Y32A7Y146BS0GA"
title = "Flaky conformance selftest under load (conformance_corpus_matches_the_spec)"
type = "bug"
category = "in-progress"
priority = "medium"
points = 3
reporter = "Claude"
created = "2026-10-14T11:50:39Z"
updated = "2026-10-15T08:50:13Z"
scope = ["crates/cadetta-cli/tests/conformance.rs", "crates/cadetta-cli/tests/conformance/**", "tests/conformance/README.md"]

[[acceptance]]
text = "Concurrent fixture writes and adapter spawns in the conformance binary are ordered so exec cannot fail with ETXTBSY"
bound = true
+++
