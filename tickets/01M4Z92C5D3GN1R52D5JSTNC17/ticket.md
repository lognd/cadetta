+++
id = "01M4Z92C5D3GN1R52D5JSTNC17"
title = 'sw-equations trailing comment drops @"..." quoting of the citation'
type = "bug"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-15T07:57:44Z"
updated = "2026-10-15T07:57:44Z"
scope = ["crates/cadetta-emit*/**"]
+++

found while working ~6ES67RW: sw-equations writes ' 1/4in emit=mid ASTM B209' but emitters/sw-equations.md 1 says citation as written in source (@"ASTM B209"); csv and json already do. Fixture emitters/sw-equations/comment-trailing-order-and-citation.
