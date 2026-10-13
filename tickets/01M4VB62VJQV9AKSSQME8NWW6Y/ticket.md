+++
id = "01M4VB62VJQV9AKSSQME8NWW6Y"
title = "Typed IEEE citations: syntax, sema, IR, digests, std and golden port"
type = "story"
category = "todo"
priority = "medium"
points = 8
parent = "01M4VB1MJC8NTMD6AAZR5Y5J08"
reporter = "lognd"
created = "2026-10-13T19:17:45Z"
updated = "2026-10-13T19:17:45Z"
persona = "design engineer"
capability = "cite sources as typed records that someone else can look up"
outcome_text = "every value says where it came from precisely"
idempotency_key = "d15:cite-core"
scope = ["crates/cadetta-syntax/src/**", "crates/cadetta-syntax/tests/**", "crates/cadetta-sema/src/cite.rs", "crates/cadetta-sema/src/lower.rs", "crates/cadetta-sema/src/resolve.rs", "crates/cadetta-ir/src/**", "docs/schemas/cadetta.ir.1.json", "std/src/**", "tests/golden/**", "tests/conformance/language/**"]

[[acceptance]]
text = "Given cite NAME: TYPE { fields } with the language.md 3.9 types and fields, when compiled, then Project.citations holds {path, type, fields, file_digest?, digest} sorted by path and citing nodes carry a CiteRef (language.md 3.9; ir.md 3)"
bound = false

[[acceptance]]
text = 'Given an unknown type or field, a malformed date or URL or a wrongly shaped value, when compiled, then E006; a repeated field is E003; the draft 1.4 forms cite x = "..." and cite x = std::... are E001 with the typed-form message (language.md 3.9)'
bound = false

[[acceptance]]
text = """Given @std::iso273, @refs::x, @"note" and @measured:"...", when lowered, then CiteRef cite, builtin, text and locator are set per ir.md 3 and a member citation replaces the component's, never merges (language.md 3.9)"""
bound = false

[[acceptance]]
text = "Given a citation field or file edit, when digests are computed, then Citation.digest changes (absent marker for a missing file) and no citing node's own or closure digest changes (ir.md 3, 4)"
bound = false

[[acceptance]]
text = "Given std/src and tests/golden, when parsed and checked, then every cite is typed, std cites are standard or book records with organization, number, title and date or edition, golden cases declare spec 1.5, and solver citations are left for the solver ticket (stdlib.md intro)"
bound = false

[[acceptance]]
text = "Given tests/conformance/language, when the corpus runs, then must-fire and must-not-fire fixtures cover each rule above"
bound = false
+++
