+++
id = "01M4VQXN3J51T9B0C08TQMFETK"
title = "cadetta emit runs the built-in json and csv adapters and writes build/<name>.<ext>"
type = "bug"
category = "todo"
priority = "medium"
points = 5
parent = "01M4SPWH74K9QQS72Z9YMTPD72"
reporter = "lognd"
created = "2026-10-13T23:00:21Z"
updated = "2026-10-15T03:55:46Z"
scope = ["crates/cadetta-cli/src/**"]

[[acceptance]]
text = "cadetta emit <json|csv> writes the documented output file; the emitters/json and emitters/csv fixtures pass and their known-gaps.toml entries are removed."
bound = true
+++

found while working ~6BCVN48: config-cli.md 3.7 says json and csv are not yet run by emit; ~49 fixtures in tests/conformance/emitters/{json,csv} and adapters/ expect build/params.json, build/params.csv and default out paths build/<adapter>.<ext>.
