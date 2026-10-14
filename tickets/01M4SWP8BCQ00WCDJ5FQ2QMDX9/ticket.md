+++
id = "01M4SWP8BCQ00WCDJ5FQ2QMDX9"
title = "Module functions and reload"
type = "story"
category = "in-progress"
priority = "medium"
points = 3
parent = "01M4SV2NDDVYW3WN79XN9C707C"
reporter = "lognd"
created = "2026-10-13T05:45:12Z"
updated = "2026-10-14T05:40:19Z"
persona = "design engineer"
capability = "use cadetta helpers directly"
idempotency_key = "replan14:py-mod"
labels = ["creates:bindings/python/python/cadetta/__init__.py", "creates:bindings/python/python/cadetta/project.py", "creates:bindings/python/python/cadetta/tools.py", "creates:bindings/python/python/cadetta/models/units.py", "creates:bindings/python/python/cadetta/models/explain.py", "creates:bindings/python/scripts/gen_models.py", "creates:bindings/python/tests/test_module_functions.py"]
scope = ["bindings/python/python/cadetta/__init__.py", "bindings/python/python/cadetta/project.py", "bindings/python/python/cadetta/tools.py", "bindings/python/python/cadetta/models/units.py", "bindings/python/python/cadetta/models/explain.py", "bindings/python/scripts/gen_models.py", "bindings/python/tests/test_module_functions.py"]

[[links]]
kind = "blocked-by"
target = "01M4SV2NMPSM42YZT9MXT4YVMW"

[[acceptance]]
text = "Given units, format_source, explain, quantity and project.reload, when called, then they behave as bindings.md 3.1, 3.2 (quantity returns Result)"
bound = true
+++
