+++
id = "01M4SPWP1JK8XH8EKXK71P7EFH"
title = "Parser and AST for declarations"
type = "story"
category = "todo"
priority = "medium"
points = 8
parent = "01M4SPWG5A0FJNPKRPDDCJQZN8"
reporter = "lognd"
created = "2026-10-13T04:03:51Z"
updated = "2026-10-13T04:03:57Z"
persona = "design engineer"
capability = "declare namespace, import, ots, model, assembly, part, fit, check and waive"
outcome_text = "I can describe a design in .cdt"
labels = ["creates:crates/cadetta-syntax/src/{parser,ast}.rs"]
scope = ["crates/cadetta-syntax/src/{parser,ast}.rs"]

[[links]]
kind = "blocks"
target = "01M4SPWP4HDVWPMA4AFJ65KWPN"

[[links]]
kind = "blocks"
target = "01M4SPWP7B9ANMSNWB9SYD2T0Q"

[[acceptance]]
text = "Given the language.md section 1 example, when parsed, then the AST has 1 ots, 2 models, 1 assembly and 1 fit"
bound = false

[[acceptance]]
text = "Given a statement ending at newline or `;`, when parsed, then both terminate it, and a newline inside `( )` does not"
bound = false

[[acceptance]]
text = "Given a reserved keyword as an identifier, when parsed, then a parse error is reported"
bound = false
+++
