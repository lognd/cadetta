+++
id = "01M4ZSCT4KRVAHVKQE36A9J2KR"
title = "Fix g005-float-half-turn must_not_fire listing G008"
type = "bug"
category = "in-progress"
priority = "low"
points = 1
reporter = "lognd"
created = "2026-10-15T12:43:04Z"
updated = "2026-10-15T16:07:52Z"
idempotency_key = "g005-halfturn-fixture"
scope = ["tests/conformance/geometry/g005-float-half-turn.*"]

[[acceptance]]
text = "g005-float-half-turn must_not_fire no longer contradicts must_fire"
bound = true
+++

expect.json lists G008 in both must_fire and must_not_fire; both .cdt files use the same half-turn so G008 fires in both. Make must_not_fire consistent (found by ~GZ74Z9Y).
