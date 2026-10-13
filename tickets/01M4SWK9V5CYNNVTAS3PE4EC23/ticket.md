+++
id = "01M4SWK9V5CYNNVTAS3PE4EC23"
title = "cadetta.diag/1 record, schema and deterministic order"
type = "story"
category = "in-progress"
priority = "medium"
points = 5
parent = "01M4SPWH0ZE4G7KXRDA5M71GPS"
reporter = "lognd"
created = "2026-10-13T05:43:35Z"
updated = "2026-10-13T06:33:04Z"
persona = "design engineer"
capability = "get one diagnostic shape from every component"
idempotency_key = "replan14:lint-diag"
labels = ["creates:crates/cadetta-lint/src/diag.rs", "creates:docs/schemas/diag.json"]
scope = ["crates/cadetta-lint/src/diag.rs", "docs/schemas/diag.json", "crates/cadetta-lint/Cargo.toml", "crates/cadetta-lint/src/lib.rs", "Cargo.toml", "Cargo.lock"]

[[links]]
kind = "blocked-by"
target = "01M4SPWHPHR3VVX5NCSXF4502V"

[[acceptance]]
text = "Given a diagnostic, when serialized, then it has schema, code, severity, path, alt, message, location, notes, fixes and waived (lints.md 1.1)"
bound = false

[[acceptance]]
text = "Given locations, when produced, then kind is source {file,start,end}, native {adapter,file,locator} or lock {line}, and design IR diagnostics always have a source span (lints.md 1.1)"
bound = false

[[acceptance]]
text = "Given a run, when sorted, then records are ordered by (file, line, code, path) and the schema is generated from the Rust type (lints.md 1.1)"
bound = false

[[acceptance]]
text = "Given solver and adapter responses, when received, then they reuse the record unchanged (lints.md 1.1; adapters.md 4)"
bound = false
+++
