+++
id = "01M4V1HE4FJ8RP212DQQ6AFN5S"
title = "Quantities: package kinds (kind Stress = Pressure) are not distinct, no Q003"
type = "bug"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T16:29:12Z"
updated = "2026-10-15T02:15:39Z"
idempotency_key = "Z48-pkgkind"
scope = ["crates/cadetta-sema/**", "crates/cadetta-syntax/src/parser.rs", "crates/cadetta-cli/src/pipeline.rs", "crates/cadetta-cli/tests/quantity_diag_once.rs", "tests/conformance/**", "docs/spec/quantities.md", "changelog.d/**"]

[[acceptance]]
text = "Package-declared kinds are distinct Q003 targets (q003-package-kind and kind-row-* fixtures pass)"
bound = true
+++

found while working ~Z48GXKX. language.md 3.10, quantities.md 4.1 kind table. Fixtures q003-package-kind and all kind-row-*.
