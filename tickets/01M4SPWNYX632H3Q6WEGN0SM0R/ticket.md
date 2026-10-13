+++
id = "01M4SPWNYX632H3Q6WEGN0SM0R"
title = "Lexer with single-token quantity literals"
type = "story"
category = "in-progress"
priority = "medium"
points = 8
parent = "01M4SPWG5A0FJNPKRPDDCJQZN8"
reporter = "lognd"
created = "2026-10-13T04:03:51Z"
updated = "2026-10-13T06:15:03Z"
persona = "design engineer"
capability = 'write literals like `5in`, `12.7mm`, `3.2N*m` and `5"`'
outcome_text = "units are unambiguous in source"
labels = ["creates:crates/cadetta-syntax/src/lexer.rs"]
scope = ["crates/cadetta-syntax/src/lexer.rs"]

[[links]]
kind = "blocks"
target = "01M4SPWP1JK8XH8EKXK71P7EFH"

[[acceptance]]
text = "Given `5 mm`, when lexed, then Q007 quantity-literal-split is reported with a fix-it"
bound = false

[[acceptance]]
text = "Given `.5mm`, when lexed, then a lex error is reported"
bound = false

[[acceptance]]
text = 'Given `5"abc"`, when lexed, then a lex error is reported'
bound = false

[[acceptance]]
text = "Given `// c` and `/// d`, when lexed, then line and doc comments are distinguished"
bound = false

[[acceptance]]
text = """Given `2'6"`, `2ft6in` and `1h30min`, when lexed, then each is one compound literal; given unordered or non-descending segments then a lex error (quantities.md 2)"""
bound = false

[[acceptance]]
text = "Given `3/4in`, `1-1/2in` and `3 / 4in`, when lexed, then the first two are single literals and the third is three tokens (quantities.md 2)"
bound = false

[[acceptance]]
text = "Given `1e-3mm`, `9.81m/s^2` and `5%`, when lexed, then each is a single quantity literal (quantities.md 2)"
bound = false

[[acceptance]]
text = "Given a reserved keyword from the language.md 2 list as an identifier, when lexed, then it is a keyword token (language.md 2)"
bound = false
+++
