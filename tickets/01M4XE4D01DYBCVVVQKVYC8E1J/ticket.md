+++
id = "01M4XE4D01DYBCVVVQKVYC8E1J"
title = "bindings/python tests/unit/test_core.py uses 'sync' as the unknown command; sync is now a real verb"
type = "bug"
category = "in-progress"
priority = "medium"
points = 1
reporter = "lognd"
created = "2026-10-14T14:47:45Z"
updated = "2026-10-14T14:50:46Z"
scope = ["bindings/python/tests/unit/test_core.py"]

[[acceptance]]
text = "Given the unknown-command test, when it runs a verb that does not exist, then run_cli returns usage error 2"
bound = true
+++
