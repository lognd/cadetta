+++
id = "01M4SWP8D2254TD7A8558R6ZY3"
title = "Result contract"
type = "story"
category = "in-progress"
priority = "medium"
points = 3
parent = "01M4SV2NDDVYW3WN79XN9C707C"
reporter = "lognd"
created = "2026-10-13T05:45:12Z"
updated = "2026-10-14T05:30:09Z"
persona = "design engineer"
capability = "separate failures from findings"
idempotency_key = "replan14:py-result"
labels = ["creates:bindings/python/python/cadetta/result.py", "creates:bindings/python/tests/test_result.py"]
scope = ["bindings/python/python/cadetta/result.py", "bindings/python/tests/test_result.py"]

[[links]]
kind = "blocked-by"
target = "01M4SV2NMPSM42YZT9MXT4YVMW"

[[acceptance]]
text = "Given a design with error diagnostics, when check runs, then Ok(report) is returned; Err is only I/O, configuration or usage (exit 2) (bindings.md 3)"
bound = true
+++
