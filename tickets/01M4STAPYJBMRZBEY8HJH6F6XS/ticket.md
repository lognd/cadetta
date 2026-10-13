+++
id = "01M4STAPYJBMRZBEY8HJH6F6XS"
title = "Conformance corpus (tests/conformance)"
type = "epic"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T05:03:57Z"
updated = "2026-10-13T05:03:57Z"
idempotency_key = "replan13:Conformance corpus (tests/conformance)"
labels = ["creates:tests/conformance/**"]
scope = ["tests/conformance/**"]
+++

Outcome: every normative rule and diagnostic of the spec has a must-fire and must-not-fire fixture, so any implementation can be tested against the spec.

Layout convention: tests/conformance/<doc>/NAME.must-fire.cdt and NAME.must-not-fire.cdt with NAME.expect.json listing expected diagnostic codes; IR and binding fixtures are JSON where a rule is not source-level.
