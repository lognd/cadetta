+++
id = "01M4SWP8F4CFX99NXDA0Y17KVQ"
title = "Models generated from schemas including diag"
type = "story"
category = "in-progress"
priority = "medium"
points = 5
parent = "01M4SV2NDDVYW3WN79XN9C707C"
reporter = "lognd"
created = "2026-10-13T05:45:12Z"
updated = "2026-10-14T12:36:44Z"
persona = "design engineer"
capability = "get typed results"
idempotency_key = "replan14:py-models"
labels = ["creates:bindings/python/python/cadetta/models/**"]
scope = ["bindings/python/python/cadetta/models/**"]

[[links]]
kind = "blocked-by"
target = "01M4SPWQANZ48Y8HG2X84RNK2D"

[[links]]
kind = "blocked-by"
target = "01M4SWK9V5CYNNVTAS3PE4EC23"

[[acceptance]]
text = "Given docs/schemas, when models are generated, then Member, Value (with dim), Quantity and the diag record are pydantic models field for field (bindings.md 3.1, 3.2)"
bound = false
+++
