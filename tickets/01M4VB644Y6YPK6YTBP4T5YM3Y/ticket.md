+++
id = "01M4VB644Y6YPK6YTBP4T5YM3Y"
title = "CLI verbs names and bind"
type = "story"
category = "in-progress"
priority = "medium"
points = 5
parent = "01M4VB1MJC8NTMD6AAZR5Y5J08"
reporter = "lognd"
created = "2026-10-13T19:17:47Z"
updated = "2026-10-13T22:34:12Z"
persona = "design engineer"
capability = "let tools ask cadetta for created names and write bindings through it"
outcome_text = "mangling and the bindings format have one implementation"
idempotency_key = "d15:names-bind"
labels = ["creates:crates/cadetta-cli/src/cmd/names.rs", "creates:crates/cadetta-cli/src/cmd/bind.rs", "creates:docs/schemas/names.json", "creates:docs/schemas/bind.json"]
scope = ["crates/cadetta-cli/src/cmd/names.rs", "crates/cadetta-cli/src/cmd/bind.rs", "crates/cadetta-cli/src/cmd/mod.rs", "crates/cadetta-cli/src/lib.rs", "docs/schemas/names.json", "docs/schemas/bind.json", "tests/conformance/config-cli/**", "crates/cadetta-cli/src/cmd/names.rs", "crates/cadetta-cli/src/cmd/bind.rs", "docs/schemas/names.json", "docs/schemas/bind.json", "crates/cadetta-cli/src/cmd/emit.rs", "crates/cadetta-cli/src/cmd/import.rs", "crates/cadetta-adapter/src/bindings.rs", "docs/spec/config-cli.md", "design/model.grmb"]

[[links]]
kind = "blocked-by"
target = "01M4VB640810PZEZHQJ1T5WFTK"

[[acceptance]]
text = "Given cadetta names [ADAPTER-ID], when run, then every in-scope bindable path is listed by canonical path with kind, emit value, tolerance, citation, doc, created name and state explicit, implicit (reason), unbound or unknown (config-cli.md 3)"
bound = true

[[acceptance]]
text = "Given cadetta bind and bind --remove, when run, then one explicit locator is added or removed in canonical order, validated per adapters.md 3, and repeating it changes nothing (config-cli.md 3)"
bound = true
+++
