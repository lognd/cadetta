+++
id = "01M4SV2NRTBH99MYAY6EWBDP66"
title = "Python query methods check, impact, impact_swap, compare, why"
type = "story"
category = "todo"
priority = "medium"
points = 5
parent = "01M4SV2NDDVYW3WN79XN9C707C"
reporter = "lognd"
created = "2026-10-13T05:17:02Z"
updated = "2026-10-13T05:17:02Z"
persona = "Python user"
capability = "run queries and get typed results"
outcome_text = "CI scripts can gate on impact and compare"
labels = ["creates:bindings/python/python/cadetta/queries.py", "creates:bindings/python/tests/test_queries.py"]
scope = ["bindings/python/python/cadetta/queries.py", "bindings/python/tests/test_queries.py"]

[[acceptance]]
text = "Given project.check, impact, impact_swap, compare and why, when called, then each returns pydantic models generated from the CLI schemas (impact.json, compare.json, diagnostic) (bindings.md 3.2)"
bound = false

[[acceptance]]
text = "Given a diagnostic, when read, then code, severity, path, span and message are present (bindings.md 3.2)"
bound = false
+++
