+++
id = "01M4ST7A8ZG0BS11X26BB84S84"
title = "Parser for interfaces, tables, processes, solvers and declarations"
type = "story"
category = "in-progress"
priority = "medium"
points = 8
parent = "01M4SPWG5A0FJNPKRPDDCJQZN8"
reporter = "lognd"
created = "2026-10-13T05:02:06Z"
updated = "2026-10-13T06:50:46Z"
persona = "cadetta contributor"
capability = "parse the remaining item kinds of the grammar"
idempotency_key = "replan13:Parser for interfaces, tables, processes, solvers and declarations"
labels = ["creates:crates/cadetta-syntax/src/parser_items.rs"]
scope = ["crates/cadetta-syntax/src/parser_items.rs", "crates/cadetta-syntax/src/parser.rs", "crates/cadetta-syntax/src/ast.rs", "docs/spec/language.md"]

[[acceptance]]
text = "Given `interface Caster { diameter: Length  load_rating >= 400N }`, when parsed, then members and bounds are recorded (language.md 3.2, 3.4)"
bound = true

[[acceptance]]
text = "Given `ots Vendor4471B: Caster @vendor { ... }` and an `: A, B` iface list, when parsed, then implements is recorded (language.md 3)"
bound = true

[[acceptance]]
text = "Given a table with `key`, `cols`, `=>` rows and key-only rows, when parsed, then rows are recorded; a row without `=>` is key-only (language.md 3, 3.6)"
bound = true

[[acceptance]]
text = 'Given `process Fdm @measured:"..." { hole_comp = ... rule hole(d: Length) -> Length = d + hole_comp }`, when parsed, then params and rules are recorded (language.md 3.8)'
bound = true

[[acceptance]]
text = "Given `solver`, `waive ... because ... expires ...`, `cite`, `unit`, `kind`, `enum`, `fn`, `fit`, `require` and `check` items, when parsed, then each production of the grammar is accepted (language.md 3)"
bound = true

[[acceptance]]
text = "Given a component with parameters `(wheel: Caster, span: Length = 1mm)`, when parsed, then defaults are recorded (language.md 3.1)"
bound = true

[[acceptance]]
text = "Given kind expressions such as `out q: Volume / Time`, when parsed, then they are accepted wherever a kind is (language.md 3.10)"
bound = false
+++
