+++
id = "01M4SXRFXB1JYNPNKSAY6WB3QC"
title = "Use grammar and visibility modifiers in the parser"
type = "story"
category = "in-progress"
priority = "medium"
points = 5
parent = "01M4SXQPADRSNTEY3J7PH899Y8"
reporter = "lognd"
created = "2026-10-13T06:03:54Z"
updated = "2026-10-13T07:21:46Z"
persona = "library author"
capability = "write use trees, aliases, globs and pub use"
labels = ["creates:crates/cadetta-syntax/src/parser_use.rs"]
scope = ["crates/cadetta-syntax/src/parser_use.rs"]

[[links]]
kind = "blocked-by"
target = "01M4SPWP1JK8XH8EKXK71P7EFH"

[[acceptance]]
text = "Given `use package::frame::Frame`, `use self::x`, `use super::super::y` and `use acme.shop::policy::AcmeShop`, when parsed, then each usepath start (package, self, super chain, dotted dependency or std) is recorded (packages.md 4; language.md 3)"
bound = false

[[acceptance]]
text = "Given `use path as alias`, `use path::*` and `use path::{A, B as C, sub::{D, *}}`, when parsed, then aliases, globs and nested trees are recorded (packages.md 4)"
bound = false

[[acceptance]]
text = "Given `pub`, `pub(package)`, `pub use` and `priv row = Iso4762Head[thread]`, when parsed, then visibility is recorded on items, uses and members (packages.md 3, 4)"
bound = false

[[acceptance]]
text = "Given no `mod` declaration or namespace header in a file, when parsed, then none is expected and a `namespace` header is a parse error (packages.md 2)"
bound = false
+++
