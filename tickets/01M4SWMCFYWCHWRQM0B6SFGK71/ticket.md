+++
id = "01M4SWMCFYWCHWRQM0B6SFGK71"
title = "Session methods open, save, close and per-locator push status"
type = "story"
category = "in-progress"
priority = "medium"
points = 5
parent = "01M4SPWH74K9QQS72Z9YMTPD72"
reporter = "lognd"
created = "2026-10-13T05:44:11Z"
updated = "2026-10-13T13:41:34Z"
persona = "design engineer"
capability = "drive a live CAD session"
idempotency_key = "replan14:ad-session"
labels = ["creates:crates/cadetta-adapter/src/session.rs"]
scope = ["crates/cadetta-adapter/src/session.rs", "crates/cadetta-adapter/src/lib.rs"]

[[links]]
kind = "blocked-by"
target = "01M4SPWRS0CDJT4GYC92AB1HCD"

[[acceptance]]
text = "Given a drive adapter, when sync runs, then the host calls open, push, save per [[adapter]] save, close and shutdown in order (adapters.md 2.2, 5)"
bound = true

[[acceptance]]
text = "Given a push response, when parsed, then each locator has its own status (adapters.md 5)"
bound = true
+++
