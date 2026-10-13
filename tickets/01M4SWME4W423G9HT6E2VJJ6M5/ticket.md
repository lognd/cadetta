+++
id = "01M4SWME4W423G9HT6E2VJJ6M5"
title = "Read cache keyed by file digest"
type = "story"
category = "todo"
priority = "medium"
points = 2
parent = "01M4SV2TCZM23R46EM75RYSA3B"
reporter = "lognd"
created = "2026-10-13T05:44:13Z"
updated = "2026-10-13T05:44:13Z"
persona = "design engineer"
capability = "make check fast"
idempotency_key = "replan14:sw-cache"
labels = ["creates:adapters/solidworks/src/cache.py"]
scope = ["adapters/solidworks/src/cache.py"]

[[acceptance]]
text = "Given an unchanged file digest, when read runs again, then the cached observed IR is returned without opening the file (adapters.md 2.2)"
bound = false
+++
