+++
id = "01M4SWP896V9Y8K5H3EJKWR1T0"
title = "Mutation and IO methods"
type = "story"
category = "in-progress"
priority = "medium"
points = 5
parent = "01M4SV2NDDVYW3WN79XN9C707C"
reporter = "lognd"
created = "2026-10-13T05:45:12Z"
updated = "2026-10-14T12:48:08Z"
persona = "design engineer"
capability = "script the whole workflow"
idempotency_key = "replan14:py-mut"
labels = ["creates:bindings/python/python/cadetta/project_ops.py", "creates:bindings/python/python/cadetta/models/{update,ack,emit,import_}.py", "creates:bindings/python/tests/test_project_ops.py"]
scope = ["bindings/python/python/cadetta/project_ops.py", "bindings/python/scripts/gen_models.py", "bindings/python/python/cadetta/models/{update,ack,emit,import_}.py", "bindings/python/python/cadetta/project.py", "docs/spec/bindings.md", "bindings/python/tests/test_project_ops.py"]

[[links]]
kind = "blocked-by"
target = "01M4SV2NMPSM42YZT9MXT4YVMW"

[[links]]
kind = "blocked-by"
target = "01M4SWN8G3QTQC5YK07VN1YQYC"

[[links]]
kind = "blocked-by"
target = "01M4SXRFQ0RENW7YM154W0EWP9"

[[acceptance]]
text = "Given solve, emit, sync, import_ and ack, when called, then each returns Result[Model, CadettaError] with the verb's JSON schema model (bindings.md 3.2)"
bound = false

[[acceptance]]
text = "Given init, cite pin and lock verbs, when looked up, then they are CLI-only (bindings.md 3.2)"
bound = false

[[acceptance]]
text = "Given project.update(name=None), when called, then it runs `update [NAME]` and returns the update.json model as a Result (bindings.md 3.2)"
bound = false
+++
