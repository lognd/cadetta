+++
id = "01M4SV2P2ZX7BMWKB98CJ14FY0"
title = "Bridge Rust tracing to Python logging"
type = "story"
category = "in-progress"
priority = "medium"
points = 3
parent = "01M4SV2NDDVYW3WN79XN9C707C"
reporter = "lognd"
created = "2026-10-13T05:17:02Z"
updated = "2026-10-14T06:07:28Z"
persona = "Python user"
capability = "see cadetta core logs through the logging module"
outcome_text = "diagnostics integrate with my application"
labels = ["creates:bindings/python/src/logging.rs", "creates:bindings/python/python/cadetta/_logging.py", "creates:bindings/python/tests/test_logging_bridge.py"]
scope = ["bindings/python/src/logging.rs", "bindings/python/python/cadetta/_logging.py", "bindings/python/Cargo.toml", "bindings/python/src/lib.rs", "bindings/python/python/cadetta/_core.pyi", "bindings/python/python/cadetta/__init__.py", "bindings/python/tests/test_logging_bridge.py", "crates/cadetta-log/src/lib.rs"]

[[links]]
kind = "blocked-by"
target = "01M4SV2NMPSM42YZT9MXT4YVMW"

[[acceptance]]
text = "Given a Python logger configured at DEBUG, when a core call runs, then tracing events arrive as logging records with level and target preserved (bindings.md 3)"
bound = true
+++
