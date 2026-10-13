+++
id = "01M4SPWS56AF9GE891GAM5QSWX"
title = "Built-in JSON adapter"
type = "story"
category = "in-progress"
priority = "medium"
points = 3
parent = "01M4SPWH74K9QQS72Z9YMTPD72"
reporter = "lognd"
created = "2026-10-13T04:03:55Z"
updated = "2026-10-13T12:50:01Z"
persona = "design engineer"
capability = "emit and read cadetta.ir/1 JSON"
outcome_text = "I can diff and consume the IR from scripts"
labels = ["creates:crates/cadetta-adapter/src/builtin/json.rs"]
scope = ["crates/cadetta-adapter/src/builtin/json.rs"]

[[links]]
kind = "blocks"
target = "01M4SPWTA1PQV15S63741V30XT"

[[acceptance]]
text = "Given the same IR twice, when emitted, then the bytes are identical (golden file)"
bound = false

[[acceptance]]
text = "Given emitted JSON, when read, then the observed IR equals the design IR values"
bound = false

[[acceptance]]
text = "Given the JSON emitter, when run, then the output is cadetta.params/1 with decimal-string values, unit and note, the header excluded from change detection, and no ref mode (emitters/json.md)"
bound = false
+++
