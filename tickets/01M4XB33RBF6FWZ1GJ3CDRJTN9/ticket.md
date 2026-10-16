+++
id = "01M4XB33RBF6FWZ1GJ3CDRJTN9"
title = "Python binding suite replays the 35 kind=binding entry=python conformance fixtures (follow-up to 6ZMPC6J)"
type = "task"
category = "in-progress"
priority = "low"
points = 5
reporter = "lognd"
created = "2026-10-14T13:54:37Z"
updated = "2026-10-16T01:21:07Z"
scope = ["bindings/python/tests/test_binding_fixtures.py", "tests/conformance/bindings/**", "tests/conformance/known-gaps.toml", "tests/conformance/README.md"]

[[acceptance]]
text = "Every kind=binding entry=python fixture under tests/conformance/bindings is replayed by the Python binding suite (bindings/python/tests/test_binding_fixtures.py) in both must_fire and must_not_fire sides, asserting its expect.json block; genuine divergences are tracked xfails bound to follow-up tickets"
bound = true
+++
