+++
id = "01M4SWFX0C2BXQGR8WQ2VN1G88"
title = "Parse datum, 'on', 'in' and place statements"
type = "story"
category = "in-progress"
priority = "medium"
points = 5
parent = "01M4SWEQMQ8DQMN8B8GWVR4GDH"
reporter = "lognd"
created = "2026-10-13T05:41:44Z"
updated = "2026-10-13T07:00:57Z"
persona = "design engineer"
capability = "write datum, point and placement syntax"
idempotency_key = "replan14:geo-parse"
labels = ["creates:crates/cadetta-syntax/src/geometry.rs", "creates:crates/cadetta-syntax/tests/geometry.rs"]
scope = ["crates/cadetta-syntax/src/geometry.rs", "crates/cadetta-syntax/tests/geometry.rs", "crates/cadetta-syntax/src/ast.rs", "crates/cadetta-syntax/src/parser.rs", "crates/cadetta-syntax/src/format.rs", "docs/spec/language.md", "docs/spec/geometry.md", "crates/cadetta-sema/src/resolve.rs"]

[[links]]
kind = "blocked-by"
target = "01M4SPWP1JK8XH8EKXK71P7EFH"

[[links]]
kind = "blocked-by"
target = "01M4SPWP4HDVWPMA4AFJ65KWPN"

[[acceptance]]
text = "Given 'datum top = origin.offset(z = height)', 'Point2 on top', 'p in D' and 'place A on B for i in 0..count(P)', when parsed, then each yields an AST node with spans (geometry.md 2, 3, 4)"
bound = false

[[acceptance]]
text = "Given 'for i in 0..N' versus 'IDENT in EXPR', when parsed, then 'IDENT in' starts a bound only when the right side is a range (geometry.md 4)"
bound = false

[[acceptance]]
text = "Given a member named origin, when parsed, then it is an error because origin is reserved (geometry.md 2)"
bound = false
+++
