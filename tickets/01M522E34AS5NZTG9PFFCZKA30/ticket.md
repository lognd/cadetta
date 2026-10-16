+++
id = "01M522E34AS5NZTG9PFFCZKA30"
title = "Shaft.seat_a.offset.z fails to evaluate in golden 02-bearing-housing (EVAL)"
type = "bug"
category = "in-progress"
priority = "medium"
points = 2
reporter = "lognd"
created = "2026-10-16T09:59:32Z"
updated = "2026-10-16T13:06:10Z"
scope = ["crates/cadetta-sema/src/geom.rs", "tests/conformance/geometry/coaxial-parameterized-model.*", "docs/spec/geometry.md"]

[[acceptance]]
text = 'A check on datums (coaxial, aligned) inside a parameterized model evaluates in its instance, with no EVAL "failed to evaluate" on the model-level members'
bound = true
+++

found while working ~NKXYVWC: tests/golden/02-bearing-housing line 64 yields EVAL 'Shaft.seat_a.offset.z failed to evaluate'; also axial_play tolerance propagation mismatches expected values (golden strict).
