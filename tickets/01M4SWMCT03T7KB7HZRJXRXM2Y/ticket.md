+++
id = "01M4SWMCT03T7KB7HZRJXRXM2Y"
title = "Import --accept, adopted record and R006"
type = "story"
category = "todo"
priority = "medium"
points = 5
parent = "01M4SPWH74K9QQS72Z9YMTPD72"
reporter = "lognd"
created = "2026-10-13T05:44:11Z"
updated = "2026-10-13T05:44:11Z"
persona = "design engineer"
capability = "adopt without re-typing"
idempotency_key = "replan14:ad-imp-acc"
labels = ["creates:crates/cadetta-adapter/src/import/accept.rs"]
scope = ["crates/cadetta-adapter/src/import/accept.rs"]

[[acceptance]]
text = "Given a proposal, when --accept runs, then bindings/ and cad/imported/ are written and an adopted lock record is added (adapters.md 7.4)"
bound = false

[[acceptance]]
text = "Given an adopted binding with a native expression, when the next push would replace it, then R006 fires (adapters.md 7.4; lints.md 3 R)"
bound = false
+++
