+++
id = "01M4V4G8YTDKDK5AB60EB99AA4"
title = "Conformance runner does not discover nested doc directories (emitters/*)"
type = "bug"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T17:20:59Z"
updated = "2026-10-13T17:21:03Z"
scope = ["crates/cadetta-cli/tests/conformance/**"]
+++

found while working ~KW6EQ0D: run_corpus lists only direct children of tests/conformance, so tests/conformance/emitters/sw-equations/*.expect.json (and csv, json) are never discovered. Recurse into nested doc dirs (doc = emitters/sw-equations); entries other than check stay skipped under ~6BCVN48.
