+++
id = "01M4V4G8YTDKDK5AB60EB99AA4"
title = "Conformance runner does not discover nested doc directories (emitters/*)"
type = "bug"
category = "in-progress"
priority = "medium"
points = 2
reporter = "lognd"
created = "2026-10-13T17:20:59Z"
updated = "2026-10-13T17:30:57Z"
scope = ["crates/cadetta-cli/tests/conformance/**", "tests/conformance/known-gaps.toml", "crates/cadetta-cli/tests/conformance.rs", "tests/conformance/README.md"]

[[acceptance]]
text = "run_corpus discovers fixtures in nested doc directories such as emitters/sw-equations and runs their check entries"
bound = true

[[acceptance]]
text = "tests/conformance/known-gaps.toml lists every newly discovered check fixture that fails, each with a ticket"
bound = true
+++

found while working ~KW6EQ0D: run_corpus lists only direct children of tests/conformance, so tests/conformance/emitters/sw-equations/*.expect.json (and csv, json) are never discovered. Recurse into nested doc dirs (doc = emitters/sw-equations); entries other than check stay skipped under ~6BCVN48.
