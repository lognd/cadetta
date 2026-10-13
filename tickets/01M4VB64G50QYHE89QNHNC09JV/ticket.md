+++
id = "01M4VB64G50QYHE89QNHNC09JV"
title = "Python binding: citation, cite_list, names, bind"
type = "story"
category = "todo"
priority = "medium"
points = 3
parent = "01M4VB1MJC8NTMD6AAZR5Y5J08"
reporter = "lognd"
created = "2026-10-13T19:17:47Z"
updated = "2026-10-13T19:19:05Z"
persona = "design engineer"
capability = "use citations, names and bindings from Python"
outcome_text = "the binding matches the CLI"
idempotency_key = "d15:py-surface"
scope = ["bindings/python/python/**", "bindings/python/tests/**", "bindings/python/src/**"]

[[links]]
kind = "blocked-by"
target = "01M4VB63RTQENVNZ4DMXBFGJX3"

[[links]]
kind = "blocked-by"
target = "01M4VB644Y6YPK6YTBP4T5YM3Y"

[[acceptance]]
text = "Given the Python package, when design.citation, project.cite_list, project.names, project.bind and project.unbind are called, then they return the CLI's JSON models as Results (bindings.md 3.1, 3.2)"
bound = false
+++
