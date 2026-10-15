+++
id = "01M4Z92C5D3GN1R52D5JSTNC17"
title = 'sw-equations trailing comment drops @"..." quoting of the citation'
type = "bug"
category = "in-progress"
priority = "medium"
points = 1
reporter = "lognd"
created = "2026-10-15T07:57:44Z"
updated = "2026-10-15T08:13:00Z"
scope = ["crates/cadetta-cli/src/cmd/emit.rs", "docs/spec/emitters/sw-equations.md", "tests/conformance/known-gaps.toml"]

[[acceptance]]
text = 'sw-equations trailing comment writes the citation as written in source (@"...")'
bound = true
+++

found while working ~6ES67RW: sw-equations writes ' 1/4in emit=mid ASTM B209' but emitters/sw-equations.md 1 says citation as written in source (@"ASTM B209"); csv and json already do. Fixture emitters/sw-equations/comment-trailing-order-and-citation.
