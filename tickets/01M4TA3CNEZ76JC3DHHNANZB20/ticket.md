+++
id = "01M4TA3CNEZ76JC3DHHNANZB20"
title = "Golden packages fail in parser/sema"
type = "bug"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T09:39:34Z"
updated = "2026-10-13T09:39:34Z"
scope = ["crates/cadetta-syntax"]
+++

found while working ~E631Y52. Goldens 01-08 produce E001/E002/EVAL under cadetta check: std names Iso4762Len etc, '@' citations after statements, 'in' as a name, '%' ops.
