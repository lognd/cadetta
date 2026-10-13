+++
id = "01M4VK19VBKCP3E55Y81KGXRMC"
title = "Python binding IR models are stale after typed citations (CI red)"
type = "bug"
category = "in-progress"
priority = "high"
points = 2
parent = "01M4SV2NDDVYW3WN79XN9C707C"
reporter = "lognd"
created = "2026-10-13T21:34:57Z"
updated = "2026-10-13T21:41:53Z"
idempotency_key = "d15:py-models-citeref"
scope = ["bindings/python/python/**", "bindings/python/tests/**"]

[[acceptance]]
text = "Given docs/schemas/cadetta.ir.1.json after ~E8NWW6Y, when cadetta.models.ir is regenerated with datamodel-codegen, then project.evaluate() parses IR with CiteRef citations and Project.citations, and the binding (python) CI job is green"
bound = true
+++

CI job binding (python) on main fails in tests/test_project.py: the generated pydantic models still expect the draft 1.4 Citation {name, target, kind} on members and reject {'builtin': 'measured'}.
