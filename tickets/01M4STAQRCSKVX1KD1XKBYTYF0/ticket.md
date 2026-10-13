+++
id = "01M4STAQRCSKVX1KD1XKBYTYF0"
title = "Conformance fixtures for solvers"
type = "story"
category = "done"
outcome = "done"
priority = "medium"
points = 5
parent = "01M4STAPYJBMRZBEY8HJH6F6XS"
reporter = "lognd"
created = "2026-10-13T05:03:58Z"
updated = "2026-10-13T17:06:18Z"
persona = "spec author"
capability = "verify an implementation against the spec"
idempotency_key = "replan13:Conformance fixtures for solvers"
labels = ["creates:tests/conformance/solvers/**"]
scope = ["tests/conformance/solvers/**", "tests/conformance/known-gaps.toml", "docs/spec/solvers.md"]

[[links]]
kind = "blocks"
target = "01M4STCFE8P4ABFFCAFEX8DC9C"

[[acceptance]]
text = "Given docs/spec/solvers.md, when fixtures are written, then every normative rule and every diagnostic code in solvers has a must-fire and a must-not-fire fixture under tests/conformance/solvers/ (solvers.md)"
bound = true

[[acceptance]]
text = "Given ambiguities found while writing fixtures, when found, then each is filed as a ticket against the spec (docs/spec/README.md)"
bound = true
+++
