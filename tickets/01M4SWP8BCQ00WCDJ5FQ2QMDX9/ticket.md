+++
id = "01M4SWP8BCQ00WCDJ5FQ2QMDX9"
title = "Module functions and reload"
type = "story"
category = "todo"
priority = "medium"
points = 3
parent = "01M4SV2NDDVYW3WN79XN9C707C"
reporter = "lognd"
created = "2026-10-13T05:45:12Z"
updated = "2026-10-13T05:45:12Z"
persona = "design engineer"
capability = "use cadetta helpers directly"
idempotency_key = "replan14:py-mod"
labels = ["creates:bindings/python/python/cadetta/__init__.py"]
scope = ["bindings/python/python/cadetta/__init__.py"]

[[acceptance]]
text = "Given units, format_source, explain, quantity and project.reload, when called, then they behave as bindings.md 3.1, 3.2 (quantity returns Result)"
bound = false
+++
