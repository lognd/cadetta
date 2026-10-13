+++
id = "01M4SPWS7TFEF86PHX2W33Q6YX"
title = "Built-in CSV adapter"
type = "story"
category = "in-progress"
priority = "medium"
points = 3
parent = "01M4SPWH74K9QQS72Z9YMTPD72"
reporter = "lognd"
created = "2026-10-13T04:03:55Z"
updated = "2026-10-13T12:59:22Z"
persona = "design engineer"
capability = "emit and read name,value,unit rows"
outcome_text = "spreadsheets can consume parameters"
labels = ["creates:crates/cadetta-adapter/src/builtin/csv.rs"]
scope = ["crates/cadetta-adapter/src/builtin/csv.rs", "crates/cadetta-adapter/src/builtin/mod.rs"]

[[links]]
kind = "blocks"
target = "01M4SPWTA1PQV15S63741V30XT"

[[acceptance]]
text = "Given a design IR, when emitted, then each parameter is one name,value,unit row, golden-file stable"
bound = false

[[acceptance]]
text = "Given the CSV, when read, then values round-trip exactly"
bound = false

[[acceptance]]
text = "Given the CSV emitter, when run, then output is RFC 4180 with a header comment and columns name,value,unit,note (emitters/csv.md)"
bound = false
+++
