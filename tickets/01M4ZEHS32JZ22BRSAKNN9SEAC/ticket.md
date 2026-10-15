+++
id = "01M4ZEHS32JZ22BRSAKNN9SEAC"
title = "Python binding conformance: 14 test_binding_meets_fixture_expectation failures"
type = "bug"
category = "done"
outcome = "invalid"
priority = "medium"
points = 2
reporter = "lognd"
created = "2026-10-15T09:33:32Z"
updated = "2026-10-15T09:37:55Z"
idempotency_key = "py-binding-fixture-fail-14"
scope = ["bindings/python/**"]

[[acceptance]]
text = "test_binding_meets_fixture_expectation passes for all cases"
bound = false
+++

Found by ~JGA9NDJ: full python suite shows 14 failures in test_binding_meets_fixture_expectation (e.g. stdlib/iso4762-any-length exit 2, expected 0). Possibly stale compiled _core; undiagnosed.
