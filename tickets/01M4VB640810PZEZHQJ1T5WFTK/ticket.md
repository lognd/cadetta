+++
id = "01M4VB640810PZEZHQJ1T5WFTK"
title = "Native links as implicit bindings and bind = scope"
type = "story"
category = "in-progress"
priority = "medium"
points = 8
parent = "01M4VB1MJC8NTMD6AAZR5Y5J08"
reporter = "lognd"
created = "2026-10-13T19:17:47Z"
updated = "2026-10-13T21:57:35Z"
persona = "design engineer"
capability = "link a dimension in CAD without restating it in a bindings file"
outcome_text = "nothing is said twice"
idempotency_key = "d15:native-links"
scope = ["crates/cadetta-adapter/src/bindings.rs", "crates/cadetta-adapter/src/sync.rs", "crates/cadetta-adapter/src/classify.rs", "crates/cadetta-adapter/src/scope.rs", "crates/cadetta-ir/src/model.rs", "crates/cadetta-ir/src/json.rs", "crates/cadetta-ir/src/schema.rs", "docs/schemas/cadetta.ir.1.json", "crates/cadetta-cli/src/config.rs", "crates/cadetta-cli/src/cmd/init.rs", "tests/conformance/adapters/**", "tests/conformance/bindings/**", "tests/conformance/config-cli/**", "crates/cadetta-ir/src/observed.rs", "crates/cadetta-adapter/src/locator.rs", "crates/cadetta-adapter/src/import/propose.rs", "crates/cadetta-cli/src/cmd/import.rs", "crates/cadetta-cli/src/cmd/impact_swap_report.rs", "crates/cadetta-cli/src/cmd/emit.rs"]

[[links]]
kind = "blocked-by"
target = "01M4VB62VJQV9AKSSQME8NWW6Y"

[[links]]
kind = "blocked-by"
target = "01M4VB63WJ5XBJ360151Y6D9TH"

[[acceptance]]
text = "Given observed IR with a global named by an in-scope path's created name (or a was name), a dimension whose equation is exactly that global, a frame named by a datum's created name, or an instance whose reference property names a part, when bound, then the implicit locator of adapters.md 3.5 exists with Locator.implicit set and nothing is written to the bindings file"
bound = true

[[acceptance]]
text = "Given an implicit locator recorded by a sync that is later missing, when synced, then R003; an expression-driven dimension is R002; two instances naming one part are R013 (adapters.md 3.5)"
bound = true

[[acceptance]]
text = """Given [[adapter]] bind = "scope", when emitting, then every in-scope expressible path has a created global, never a let, another package's member or an inexpressible Text path (adapters.md 3.5)"""
bound = true

[[acceptance]]
text = 'Given cadetta init --adapter sw-equations, when run, then the block carries bind = "scope" (config-cli.md 3.2)'
bound = false
+++
