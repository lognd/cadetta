+++
id = "01M513E2F1HRD0J1XKATZFWFKW"
title = "ir.md variants key example is short-form but the IR emits qualified keys"
type = "bug"
category = "done"
outcome = "done"
priority = "medium"
points = 1
reporter = "lognd"
created = "2026-10-16T00:57:45Z"
updated = "2026-10-16T01:00:45Z"
scope = ["docs/spec/ir.md"]

[[acceptance]]
text = "Given ir.md, when reading the variants note, then the key example is the full qualified SLOT=ALT form the IR emits"
bound = true
+++

found while working ~QQY7ZED: cadetta ir emits 'acme.rover::Rover/wheel=acme.rover::Colson2Series' (see bindings/python/tests/fixtures/alt-wheel); ir.md line ~259 documents 'Rover/wheel=Colson2Series'. Reconcile doc or emitter.
