+++
id = "01M4VB648PMGRPG6R8D606HZK6"
title = "Drive and compare dimension tolerances"
type = "story"
category = "done"
outcome = "done"
priority = "medium"
points = 5
parent = "01M4VB1MJC8NTMD6AAZR5Y5J08"
reporter = "lognd"
created = "2026-10-13T19:17:47Z"
updated = "2026-10-13T23:00:22Z"
persona = "design engineer"
capability = "have drawing tolerances come from the design"
outcome_text = "tolerances are not retyped on drawings"
idempotency_key = "d15:dim-tolerances"
scope = ["crates/cadetta-adapter/src/sync.rs", "crates/cadetta-adapter/src/classify.rs", "crates/cadetta-adapter/src/rpc/**", "crates/cadetta-adapter/src/locator.rs", "docs/schemas/adapter-protocol.json", "tests/conformance/adapters/**", "crates/cadetta-cli/src/config.rs", "crates/cadetta-cli/src/cmd/impact_swap_report.rs", "crates/cadetta-ir/src/model.rs", "crates/cadetta-ir/src/json.rs", "crates/cadetta-ir/src/observed.rs", "docs/schemas/cadetta.ir.1.json", "crates/cadetta-adapter/src/import/propose.rs", "crates/cadetta-adapter/src/import/accept.rs", "crates/cadetta-adapter/src/session.rs", "crates/cadetta-adapter/src/bindings.rs", "crates/cadetta-cli/src/cmd/import.rs", "crates/cadetta-ir/src/schema.rs", "crates/cadetta-cli/src/cmd/bind.rs", "bindings/python/python/cadetta/models/ir.py"]

[[links]]
kind = "blocked-by"
target = "01M4VB640810PZEZHQJ1T5WFTK"

[[acceptance]]
text = "Given an adapter with the tolerances feature and a dimension locator with tolerance = drive, when pushed, then the design tolerance is sent as symmetric, bilateral, limits or fit class per adapters.md 3.6, as limits when the emit point is not nominal"
bound = true

[[acceptance]]
text = "Given a read-back tolerance that differs, when synced, then it is classified in the same three-way record as the value and reported as R001 (adapters.md 3.6)"
bound = true

[[acceptance]]
text = "Given initialize, when parsed, then the features list is accepted and validated against docs/schemas/adapter-protocol.json (adapters.md 5)"
bound = true
+++
