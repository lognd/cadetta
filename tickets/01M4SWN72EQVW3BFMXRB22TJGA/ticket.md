+++
id = "01M4SWN72EQVW3BFMXRB22TJGA"
title = "Selector grammar KIND:GLOB"
type = "story"
category = "todo"
priority = "medium"
points = 3
parent = "01M4ST8EJWTGQM9CMVBJVBATJT"
reporter = "lognd"
created = "2026-10-13T05:44:38Z"
updated = "2026-10-13T05:44:38Z"
persona = "design engineer"
capability = "pick obligations precisely"
idempotency_key = "replan14:ob-selector"
labels = ["creates:crates/cadetta-obligations/src/selector.rs"]
scope = ["crates/cadetta-obligations/src/selector.rs"]

[[acceptance]]
text = "Given KIND:GLOB[,KIND:GLOB...], when parsed, then it matches obligation ids <kind>:<canonical path>; a selector matching nothing is exit 2 (config-cli.md 3.3; obligations.md 4)"
bound = false
+++
