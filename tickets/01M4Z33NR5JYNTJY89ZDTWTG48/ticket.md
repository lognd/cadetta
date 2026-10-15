+++
id = "01M4Z33NR5JYNTJY89ZDTWTG48"
title = "F002 fixture granularity contradicts language.md 3.7 (nm vs pm)"
type = "task"
category = "todo"
priority = "medium"
points = 3
reporter = "lognd"
created = "2026-10-15T06:13:36Z"
updated = "2026-10-15T06:13:36Z"
idempotency_key = "a0fq-f002"
scope = ["tests/conformance/**"]

[[acceptance]]
text = "Decision recorded and fixture plus known-gaps entry consistent with language.md 3.7"
bound = false
+++

Split from ~A0FQTVX. Fixture language/f002-sub-nanometre: must-fire 20.0000000005mm (0.5 pm) and must-not-fire 20.000000001mm (1 pm) are both non-integer nm, so spec 3.7 (whole nm) cannot separate them; author likely read 1e-9 mm as 1 nm. Needs a human decision: spec granularity, or correct the fixture.
