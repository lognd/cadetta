+++
id = "01M4SV2P8R37D8SQA4G3AQ8XV1"
title = "Optional cadetta[pint] interop (Quantity.to_pint)"
type = "story"
category = "in-progress"
priority = "medium"
points = 3
parent = "01M4SV2NDDVYW3WN79XN9C707C"
reporter = "lognd"
created = "2026-10-13T05:17:03Z"
updated = "2026-10-13T07:44:46Z"
persona = "Python user"
capability = "convert Quantities to pint"
outcome_text = "I can use numpy and pint workflows"
labels = ["creates:bindings/python/python/cadetta/pint_interop.py", "creates:bindings/python/tests/test_pint.py"]
scope = ["bindings/python/python/cadetta/pint_interop.py", "bindings/python/tests/test_pint.py"]

[[links]]
kind = "blocked-by"
target = "01M4SV2NGV5P9KBBT7987BZCKQ"

[[acceptance]]
text = "Given the cadetta[pint] extra installed, when .to_pint(registry) is called, then an equivalent pint Quantity is returned (bindings.md 3.1)"
bound = true

[[acceptance]]
text = "Given pint is absent, when .to_pint is called, then a Result error names the missing extra (bindings.md 3.1)"
bound = true
+++
