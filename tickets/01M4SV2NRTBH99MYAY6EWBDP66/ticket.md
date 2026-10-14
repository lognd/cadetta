+++
id = "01M4SV2NRTBH99MYAY6EWBDP66"
title = "Python query methods check, impact, impact_swap, compare, why"
type = "story"
category = "in-progress"
priority = "medium"
points = 5
parent = "01M4SV2NDDVYW3WN79XN9C707C"
reporter = "lognd"
created = "2026-10-13T05:17:02Z"
updated = "2026-10-14T11:58:02Z"
persona = "Python user"
capability = "run queries and get typed results"
outcome_text = "CI scripts can gate on impact and compare"
labels = ["creates:bindings/python/python/cadetta/queries.py", "creates:bindings/python/tests/test_queries.py"]
scope = ["bindings/python/python/cadetta/queries.py", "bindings/python/tests/test_queries.py", "bindings/python/python/cadetta/project.py", "bindings/python/scripts/gen_models.py", "bindings/python/python/cadetta/models/*.py"]

[[links]]
kind = "blocked-by"
target = "01M4ST8F5G8E0FMC3KZJG8A95T"

[[links]]
kind = "blocked-by"
target = "01M4ST8F7B9VYMFSCJD8MVBJNS"

[[links]]
kind = "blocked-by"
target = "01M4ST8F91FHYFC01TN6018BAM"

[[links]]
kind = "blocked-by"
target = "01M4ST8FCNN3GZKZNRK2X3Q55S"

[[links]]
kind = "blocked-by"
target = "01M4ST8FEBCF9PC9SR0ZKTYT70"

[[links]]
kind = "blocked-by"
target = "01M4SV2NMPSM42YZT9MXT4YVMW"

[[acceptance]]
text = "Given project.check, impact, impact_swap, compare and why, when called, then each returns pydantic models generated from the CLI schemas (impact.json, compare.json, diagnostic) (bindings.md 3.2)"
bound = false

[[acceptance]]
text = "Given a diagnostic, when read, then code, severity, path, span and message are present (bindings.md 3.2)"
bound = false

[[acceptance]]
text = "Given why_stale, impact_since, obligations, swap and impact_swap, when called, then each returns its schema model (bindings.md 3.2)"
bound = false
+++
