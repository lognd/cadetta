+++
id = "01M4SWME04D18722NZS7TE82RN"
title = "Assembly read, R011 and component field"
type = "story"
category = "todo"
priority = "medium"
points = 5
parent = "01M4SV2TCZM23R46EM75RYSA3B"
reporter = "lognd"
created = "2026-10-13T05:44:13Z"
updated = "2026-10-13T05:44:15Z"
persona = "design engineer"
capability = "bind assembly instances"
idempotency_key = "replan14:sw-asm"
labels = ["creates:adapters/solidworks/src/assembly.py"]
scope = ["adapters/solidworks/src/assembly.py"]

[[links]]
kind = "blocked-by"
target = "01M4SV2TKZD26CK9FFA1EJXSV4"

[[acceptance]]
text = "Given an assembly, when read, then locators carry component and R011 fires when a slot holds a file other than its primary (adapters.md 3; lints.md 3 R)"
bound = false
+++
