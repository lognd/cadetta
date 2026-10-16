+++
id = "01M514JX5PKMQ4SEGBRKNWJCVG"
title = "Conformance runner treats kind=binding entry=python as externally replayed, then drop the 35 bindings/* known-gaps"
type = "task"
category = "in-progress"
priority = "medium"
points = 2
reporter = "lognd"
created = "2026-10-16T01:17:52Z"
updated = "2026-10-16T01:57:35Z"
scope = ["crates/cadetta-cli/tests/conformance/**", "tests/conformance/known-gaps.toml"]

[[acceptance]]
text = "Rust conformance runner skips kind=binding entry=python fixtures explicitly and the 35 bindings/* known-gaps entries are removed"
bound = true
+++

Follow-up to CDRJTN9. The Python suite bindings/python/tests/test_binding_fixtures.py now replays all 35 fixtures, but exec.rs has no (binding, python) executor so the Rust runner still needs the known-gaps entries. Add an explicit skip for entry=python and remove the 35 gaps.
