+++
id = "01M4SPWNYX632H3Q6WEGN0SM0R"
title = "Lexer with single-token quantity literals"
type = "story"
category = "todo"
priority = "medium"
points = 5
parent = "01M4SPWG5A0FJNPKRPDDCJQZN8"
reporter = "lognd"
created = "2026-10-13T04:03:51Z"
updated = "2026-10-13T04:03:51Z"
persona = "design engineer"
capability = 'write literals like `5in`, `12.7mm`, `3.2N*m` and `5"`'
outcome_text = "units are unambiguous in source"
labels = ["creates:crates/cadetta-syntax/src/lexer.rs"]
scope = ["crates/cadetta-syntax/src/lexer.rs"]

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
+++
