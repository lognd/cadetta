+++
id = "01M4YMKH7NQW964KS1C2GJX813"
title = "Quantities: user-declared kinds are not registered in the KindTable (kind-row-length-from-cbrt Q003)"
type = "bug"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-15T02:00:07Z"
updated = "2026-10-15T02:00:07Z"
scope = ["crates/cadetta-sema/**", "crates/cadetta-cli/src/pipeline.rs"]
+++

found while working ~FP8V54M. pipeline uses KindTable::builtin(); user 'kind Sink = Length' decls are never registered so kindcheck target() returns None and Q003 is not raised (fixture quantities/kind-row-length-from-cbrt).
