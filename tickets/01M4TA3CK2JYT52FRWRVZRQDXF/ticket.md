+++
id = "01M4TA3CK2JYT52FRWRVZRQDXF"
title = "cmd/solve.rs passes enums the resolver already indexes"
type = "bug"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T09:39:34Z"
updated = "2026-10-15T01:45:37Z"
scope = ["crates/cadetta-cli/src/cmd/solve.rs"]

[[acceptance]]
text = "cadetta solve passes no enum declarations to the resolver, so a module with an enum yields no duplicate E003"
bound = false
+++

found while working ~E631Y52. causes duplicate E003.
