+++
id = "01M4SWN8G3QTQC5YK07VN1YQYC"
title = "Output schemas for all commands"
type = "story"
category = "in-progress"
priority = "medium"
points = 5
parent = "01M4SPWHHNRSGB9QZ04CMHWRRB"
reporter = "lognd"
created = "2026-10-13T05:44:40Z"
updated = "2026-10-13T13:36:54Z"
persona = "design engineer"
capability = "script against stable JSON"
idempotency_key = "replan14:cli-schemas"
labels = ["creates:docs/schemas/*.json"]
scope = ["docs/schemas/*.json", "crates/cadetta-cli/src/cmd/ack.rs", "crates/cadetta-cli/src/cmd/obligations.rs"]

[[links]]
kind = "blocked-by"
target = "01M4SPWQANZ48Y8HG2X84RNK2D"

[[links]]
kind = "blocked-by"
target = "01M4SPWSXBTNXW3CPZ6FVHT5ZT"

[[links]]
kind = "blocked-by"
target = "01M4SWK9V5CYNNVTAS3PE4EC23"

[[acceptance]]
text = "Given each command in config-cli.md 3, when run with --format json, then output validates against its generated schema in docs/schemas (config-cli.md 5)"
bound = false
+++
