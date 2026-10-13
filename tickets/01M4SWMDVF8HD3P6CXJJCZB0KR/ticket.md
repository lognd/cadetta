+++
id = "01M4SWMDVF8HD3P6CXJJCZB0KR"
title = "SOLIDWORKS naming module"
type = "story"
category = "todo"
priority = "medium"
points = 3
parent = "01M4SV2TCZM23R46EM75RYSA3B"
reporter = "lognd"
created = "2026-10-13T05:44:12Z"
updated = "2026-10-13T05:44:12Z"
persona = "design engineer"
capability = "get stable equation names"
idempotency_key = "replan14:sw-naming"
labels = ["creates:adapters/solidworks/src/naming.py"]
scope = ["adapters/solidworks/src/naming.py"]

[[acceptance]]
text = "Given a canonical path, when named, then the global variable follows emitters/sw-equations.md 4 including component<n>, prefix and legal characters"
bound = false
+++
