+++
id = "01M4SV2P2ZX7BMWKB98CJ14FY0"
title = "Bridge Rust tracing to Python logging"
type = "story"
category = "todo"
priority = "medium"
points = 3
parent = "01M4SV2NDDVYW3WN79XN9C707C"
reporter = "lognd"
created = "2026-10-13T05:17:02Z"
updated = "2026-10-13T05:17:02Z"
persona = "Python user"
capability = "see cadetta core logs through the logging module"
outcome_text = "diagnostics integrate with my application"
labels = ["creates:bindings/python/src/logging.rs", "creates:bindings/python/python/cadetta/_logging.py"]
scope = ["bindings/python/src/logging.rs", "bindings/python/python/cadetta/_logging.py"]

[[acceptance]]
text = "Given a Python logger configured at DEBUG, when a core call runs, then tracing events arrive as logging records with level and target preserved (bindings.md 3)"
bound = false
+++
