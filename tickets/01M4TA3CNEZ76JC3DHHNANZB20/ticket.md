+++
id = "01M4TA3CNEZ76JC3DHHNANZB20"
title = "Golden packages fail in parser/sema"
type = "bug"
category = "todo"
priority = "medium"
points = 8
reporter = "lognd"
created = "2026-10-13T09:39:34Z"
updated = "2026-10-13T09:45:33Z"
scope = ["crates/cadetta-syntax"]

[[acceptance]]
text = "Given each tests/golden/* package, when parsed, then there are no E001 syntax errors (grammar fixes recorded in docs/spec/language.md where the golden and spec disagree)"
bound = false
+++

found while working ~E631Y52. Goldens 01-08 produce E001/E002/EVAL under cadetta check: std names Iso4762Len etc, '@' citations after statements, 'in' as a name, '%' ops.
