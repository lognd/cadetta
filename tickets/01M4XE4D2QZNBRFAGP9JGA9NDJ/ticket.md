+++
id = "01M4XE4D2QZNBRFAGP9JGA9NDJ"
title = "Python test suite hangs past 100s in test_project_ops.py (suspect real-CLI test e.g. test_update_real_cli_runs)"
type = "bug"
category = "todo"
priority = "medium"
points = 1
reporter = "lognd"
created = "2026-10-14T14:47:45Z"
updated = "2026-10-15T09:28:27Z"
scope = ["bindings/python/tests/test_conformance.py"]

[[acceptance]]
text = "A single selected conformance case replays only selected cases, not the whole corpus"
bound = false
+++

Root cause: the module-scoped replays fixture in test_conformance.py eagerly replayed all ~2700 corpus cases (a python subprocess each, ~200s) on first use, so any -k selection or run paid minutes. test_project_ops.py itself runs in 1s.
