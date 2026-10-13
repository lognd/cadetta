+++
id = "01M4V1HEA425D4RGFBSG5NT80Z"
title = "Quantities: inexact operand in check gives EVAL, spec 5.8 wants undecided/holds with 1e-9 widening"
type = "bug"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T16:29:12Z"
updated = "2026-10-13T16:29:12Z"
idempotency_key = "Z48-inexactcheck"
scope = ["crates/cadetta-lint/**"]
+++

found while working ~Z48GXKX. quantities.md 5.8. Fixture c002-inexact-boundary.
