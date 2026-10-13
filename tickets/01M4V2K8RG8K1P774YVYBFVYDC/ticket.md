+++
id = "01M4V2K8RG8K1P774YVYBFVYDC"
title = "S007 not raised for untyped solver out port"
type = "bug"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T16:47:40Z"
updated = "2026-10-13T16:47:40Z"
scope = ["crates/cadetta-sema/**"]
+++

Fixture lints/s007-solver-untyped-port: out without kind gives E001 instead of S007; in with non-quantity expression gives nothing. found while working ~T71T26K
