+++
id = "01M4SWP87FJ0AYP6MYC50SS49Y"
title = "QuantityError with Q-code"
type = "story"
category = "in-progress"
priority = "medium"
points = 3
parent = "01M4SV2NDDVYW3WN79XN9C707C"
reporter = "lognd"
created = "2026-10-13T05:45:12Z"
updated = "2026-10-13T06:50:04Z"
persona = "design engineer"
capability = "get numeric-library errors"
idempotency_key = "replan14:py-qerr"
labels = ["creates:bindings/python/python/cadetta/errors.py"]
scope = ["bindings/python/python/cadetta/errors.py"]

[[links]]
kind = "blocked-by"
target = "01M4SV2NGV5P9KBBT7987BZCKQ"

[[acceptance]]
text = "Given an operator that violates dimension or kind, when applied, then QuantityError is raised carrying the Q-code the compiler would report (bindings.md 3)"
bound = false
+++
