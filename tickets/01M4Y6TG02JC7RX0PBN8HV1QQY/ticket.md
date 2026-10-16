+++
id = "01M4Y6TG02JC7RX0PBN8HV1QQY"
title = "KindChecker: untyped locals must shadow same-named component members"
type = "bug"
category = "in-progress"
priority = "low"
points = 2
reporter = "lognd"
created = "2026-10-14T21:59:15Z"
updated = "2026-10-16T00:48:01Z"
idempotency_key = "kindcheck-untyped-local-shadow"
scope = ["crates/cadetta-sema", "docs/spec/quantities.md", "changelog.d", "crates/cadetta-cli/tests", "crates/cadetta-cli/tests/**", "crates/cadetta-sema/**"]

[[acceptance]]
text = "A for/let local of unknown type that shares a name with a component quantity member does not resolve to the member: cadetta check reports no Q diagnostic from the member's dimension"
bound = true

[[acceptance]]
text = "Typed locals and genuine member reads keep their existing behaviour (existing cross-component tests still pass)"
bound = true
+++

Follow-up to ~CH5HR2K. In kindcheck.rs, locals only holds locals whose type is known, so an untyped let/for variable sharing a name with a quantity member resolves to that member and can produce a false Q002. Record unknown-typed locals as shadows so lookup stops at them.
