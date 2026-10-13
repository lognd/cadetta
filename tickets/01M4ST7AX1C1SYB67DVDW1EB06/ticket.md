+++
id = "01M4ST7AX1C1SYB67DVDW1EB06"
title = "Attributes, citations and cite declarations"
type = "story"
category = "in-progress"
priority = "medium"
points = 3
parent = "01M4SPWGB2729B40CQE81MHM2Z"
reporter = "lognd"
created = "2026-10-13T05:02:06Z"
updated = "2026-10-13T07:54:34Z"
persona = "design engineer"
capability = "attach provenance and policy to values once"
idempotency_key = "replan13:Attributes, citations and cite declarations"
labels = ["creates:crates/cadetta-sema/src/{attrs,cite}.rs"]
scope = ["crates/cadetta-sema/src/{attrs,cite}.rs", "crates/cadetta-sema/src/lib.rs", "docs/spec/language.md", "changelog.d/01M4ST7AX1C1SYB67DVDW1EB06.added.md"]

[[acceptance]]
text = "Given `#[emit(mid)]`, `#![display(mm)]` and trailing `#[...]`, when resolved, then scope defaults apply and the member attribute wins (language.md 3.11)"
bound = true

[[acceptance]]
text = 'Given `cite name = "url-or-path"`, `@name:"locator"`, `@"literal"` and `@measured:"who, when"`, when resolved, then each citation has a kind url|file|standard|measured|assumed|estimate (language.md 3.9, ir.md 3)'
bound = true

[[acceptance]]
text = "Given a component citation, when members have none, then they inherit it; a member citation wins (language.md 3.9)"
bound = true

[[acceptance]]
text = "Given a local file citation, when resolved, then its file digest is recorded; a literal citation is never checked for reachability (language.md 3.9)"
bound = true
+++
