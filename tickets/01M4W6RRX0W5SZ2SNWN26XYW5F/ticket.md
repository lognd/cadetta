+++
id = "01M4W6RRX0W5SZ2SNWN26XYW5F"
title = "Python InitializeResult lacks the optional features field"
type = "bug"
category = "todo"
priority = "medium"
points = 1
reporter = "lognd"
created = "2026-10-14T03:19:49Z"
updated = "2026-10-14T03:19:49Z"
scope = ["bindings/python/**"]

[[acceptance]]
text = "Given a Python adapter returning features in initialize, when serialized, then features is present and validates against the adapter protocol schema; absent stays absent"
bound = false
+++

found while working ~RDPEZRQ: cadetta.adapter.InitializeResult forbids extra fields and has no features list, so Python adapters cannot report features (adapters.md initialize). Add the optional field (absent means none), from the schema via gen_models.py if the model is generated.
