+++
id = "01M4VKKB35X383AQA0SF2KGRGH"
title = "Record the Python model codegen and fail CI when models drift from docs/schemas"
type = "chore"
category = "in-progress"
priority = "medium"
points = 3
parent = "01M4SV2NDDVYW3WN79XN9C707C"
reporter = "lognd"
created = "2026-10-13T21:44:48Z"
updated = "2026-10-13T22:19:57Z"
idempotency_key = "d15:py-models-check"
labels = ["creates:bindings/python/scripts/gen_models.py", "creates:grimble.lock"]
scope = ["bindings/python/python/cadetta/models/**", ".github/workflows/ci.yml", "bindings/python/README.md", "bindings/python/scripts/gen_models.py", "design/model.grmb", "grimble.lock", "bindings/python/tests/test_gen_models.py", "bindings/python/pyproject.toml", "bindings/python/uv.lock"]

[[acceptance]]
text = "Given bindings/python/scripts/gen_models.py, when run, then it regenerates every cadetta.models module from docs/schemas with the recorded datamodel-codegen options, and --check exits non-zero when a committed model is stale; the binding CI job runs the check"
bound = true

[[acceptance]]
text = "Given design/model.grmb flow f_py_ir, when bound with the Rust IR Project type (crates/cadetta-ir/src/model.rs) as producer and bindings/python/python/cadetta/models/ir.py as consumer and acked into a committed grimble.lock, then grimble check reports SYS006 when the producer changes without the consumer; if the 0.532.0 Python adapter leaves the consumer end Unresolved, the ticket records the reason code and keeps the flow as documentation while the gen --check stays the hard gate (checked with uvx --from frob==0.532.0)"
bound = true
+++

Found by ~1KGXRMC: a schema change in a Rust ticket left the generated models stale and turned main red; the codegen options had to be reverse-engineered.
