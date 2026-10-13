+++
id = "01M4SWMCYW919R5JKSV2T1ZYBN"
title = "Frame locators and geometry bindings"
type = "story"
category = "todo"
priority = "medium"
points = 5
parent = "01M4SPWH74K9QQS72Z9YMTPD72"
reporter = "lognd"
created = "2026-10-13T05:44:11Z"
updated = "2026-10-13T05:44:11Z"
persona = "design engineer"
capability = "drive sketch geometry from the design"
idempotency_key = "replan14:ad-geo"
labels = ["creates:crates/cadetta-adapter/src/geometry_bind.rs"]
scope = ["crates/cadetta-adapter/src/geometry_bind.rs"]

[[acceptance]]
text = "Given a frame-kind locator for a datum or point coordinate, when emitted, then sign and the sketch origin convention apply (adapters.md 3.4; geometry.md 6)"
bound = false

[[acceptance]]
text = "Given a coordinate bound to a derived term that is not verified, when emitting, then only verified-only terms are written (geometry.md 6)"
bound = false
+++
