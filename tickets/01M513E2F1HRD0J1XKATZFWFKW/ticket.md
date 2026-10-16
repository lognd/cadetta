+++
id = "01M513E2F1HRD0J1XKATZFWFKW"
title = "ir.md variants key example is short-form but the IR emits qualified keys"
type = "bug"
category = "in-progress"
priority = "medium"
points = 1
reporter = "lognd"
created = "2026-10-16T00:57:45Z"
updated = "2026-10-16T00:59:44Z"
scope = ["docs/spec/ir.md"]
+++

found while working ~QQY7ZED: cadetta ir emits 'acme.rover::Rover/wheel=acme.rover::Colson2Series' (see bindings/python/tests/fixtures/alt-wheel); ir.md line ~259 documents 'Rover/wheel=Colson2Series'. Reconcile doc or emitter.
