+++
id = "01M4SV2NZT32CAT33D13T86646"
title = "Python adapter base class cadetta.adapter.Adapter with JSON-RPC loop"
type = "story"
category = "todo"
priority = "medium"
points = 8
parent = "01M4SV2NDDVYW3WN79XN9C707C"
reporter = "lognd"
created = "2026-10-13T05:17:02Z"
updated = "2026-10-13T05:17:04Z"
persona = "adapter author"
capability = "subclass Adapter and override initialize, emit, read, push, regen, shutdown"
outcome_text = "one SDK serves every Python-reachable CAD tool"
labels = ["creates:bindings/python/python/cadetta/adapter.py", "creates:bindings/python/tests/test_adapter.py"]
scope = ["bindings/python/python/cadetta/adapter.py", "bindings/python/tests/test_adapter.py"]

[[links]]
kind = "blocked-by"
target = "01M4SPWRS0CDJT4GYC92AB1HCD"

[[links]]
kind = "blocked-by"
target = "01M4SV2NMPSM42YZT9MXT4YVMW"

[[acceptance]]
text = "Given a subclass, when JSON-RPC 2.0 requests arrive on stdin, then methods dispatch and results validate against the protocol schema (bindings.md 3.4, adapters.md 5)"
bound = false

[[acceptance]]
text = "Given logging inside an adapter, when it runs, then output goes to stderr only and stdout carries only protocol (bindings.md 3.4)"
bound = false

[[acceptance]]
text = "Given an adapter, when observed IR is built, then it uses the same pydantic IR models (bindings.md 3.4)"
bound = false
+++
