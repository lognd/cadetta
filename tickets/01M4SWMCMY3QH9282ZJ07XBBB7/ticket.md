+++
id = "01M4SWMCMY3QH9282ZJ07XBBB7"
title = "Import proposal in proposal.toml and proposal.cdt"
type = "story"
category = "todo"
priority = "medium"
points = 5
parent = "01M4SPWH74K9QQS72Z9YMTPD72"
reporter = "lognd"
created = "2026-10-13T05:44:11Z"
updated = "2026-10-13T05:44:14Z"
persona = "design engineer"
capability = "adopt an existing CAD model"
idempotency_key = "replan14:ad-imp-prop"
labels = ["creates:crates/cadetta-adapter/src/import/propose.rs"]
scope = ["crates/cadetta-adapter/src/import/propose.rs"]

[[links]]
kind = "blocked-by"
target = "01M4ST7B09TGN6VSXJBZ8FDVCN"

[[acceptance]]
text = "Given observed IR, when import runs without --accept, then build/import/proposal.toml and proposal.cdt are written and nothing else (adapters.md 7.1, 7.3)"
bound = false

[[acceptance]]
text = "Given --assume-unit U, when native values lack units, then unit_assumed is recorded (adapters.md 7.1)"
bound = false
+++
