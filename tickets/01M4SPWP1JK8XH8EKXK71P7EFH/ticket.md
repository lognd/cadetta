+++
id = "01M4SPWP1JK8XH8EKXK71P7EFH"
title = "Parser and AST for components, parameters and parts"
type = "story"
category = "in-progress"
priority = "medium"
points = 8
parent = "01M4SPWG5A0FJNPKRPDDCJQZN8"
reporter = "lognd"
created = "2026-10-13T04:03:51Z"
updated = "2026-10-13T06:28:49Z"
persona = "design engineer"
capability = "declare import, ots, model, assembly, part, fit, check and waive"
outcome_text = "I can describe a design in .cdt"
labels = ["creates:crates/cadetta-syntax/src/{parser,ast}.rs"]
scope = ["crates/cadetta-syntax/src/{parser,ast}.rs", "crates/cadetta-syntax/src/lib.rs", "docs/spec/language.md"]

[[links]]
kind = "blocks"
target = "01M4SPWP4HDVWPMA4AFJ65KWPN"

[[links]]
kind = "blocks"
target = "01M4SPWP7B9ANMSNWB9SYD2T0Q"

[[links]]
kind = "blocks"
target = "01M4ST7A8ZG0BS11X26BB84S84"

[[acceptance]]
text = "Given the language.md section 1 example, when parsed, then the AST has 1 interface, 2 ots, 3 models, 1 assembly and 1 fit member (language.md 1)"
bound = true

[[acceptance]]
text = "Given the grammar in language.md 3, when parsed, then file, use, component, params, member, let, bound, part and check productions are accepted"
bound = true

[[acceptance]]
text = "Given a statement ending at newline or `;`, when parsed, then both terminate it, and a newline inside `( )` or `[ ]` does not (language.md 2)"
bound = true

[[acceptance]]
text = "Given a reserved keyword as an identifier, when parsed, then a parse error is reported (language.md 2)"
bound = true

[[acceptance]]
text = "Given `part wheel[4]: Caster = Vendor4471B alt [Colson2Series]`, when parsed, then the part has a primary and one alternate (language.md 3)"
bound = true

[[acceptance]]
text = "Given the file production, when parsed, then it is `{ use | item }` with no namespace header or mod declaration, and `use` follows packages.md 4 (language.md 3; packages.md 2)"
bound = false

[[acceptance]]
text = "Given `use std::fasteners::{MetricThread, ScrewJoint}` and `as` renames, when parsed, then the imports are recorded (language.md 3; packages.md 4)"
bound = false
+++
