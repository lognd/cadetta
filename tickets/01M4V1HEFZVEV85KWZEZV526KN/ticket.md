+++
id = "01M4V1HEFZVEV85KWZEZV526KN"
title = "Quantities: emit point not observable in conformance"
type = "bug"
category = "done"
outcome = "done"
priority = "medium"
points = 3
reporter = "lognd"
created = "2026-10-13T16:29:12Z"
updated = "2026-10-16T07:12:30Z"
idempotency_key = "Z48-emitpoint"
labels = ["creates:tests/conformance/quantities/emit-*"]
scope = ["tests/conformance/README.md", "tests/conformance/quantities/emit-*", "crates/cadetta-cli/tests/conformance.rs", "tests/conformance/quantities/emit-*/**"]

[[acceptance]]
text = "Conformance fixtures observe the emitted point (nominal default, mid, mmc, lmc, inherited by reference) through the emit entry output expectation"
bound = true

[[acceptance]]
text = "tests/conformance/README.md documents that emit-point rules are observed via kind source entry emit with output contains/excludes"
bound = true
+++

found while working ~Z48GXKX. quantities.md emit rules; no expect.json field for emitted text, so no fixture was written. Needs a conformance-surface decision.
