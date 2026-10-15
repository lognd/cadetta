+++
id = "01M4V2K8F7Q0PMTTWV48S2ESWE"
title = "Q003 does not fire for package-declared kinds"
type = "bug"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T16:47:40Z"
updated = "2026-10-15T02:15:40Z"
scope = ["crates/cadetta-lint/**", "crates/cadetta-sema/**", "crates/cadetta-syntax/src/parser.rs", "crates/cadetta-cli/src/pipeline.rs", "crates/cadetta-cli/tests/quantity_diag_once.rs", "tests/conformance/**", "docs/spec/quantities.md", "changelog.d/**"]

[[acceptance]]
text = "lints/q003-named-kind-package fires exactly one Q003 and its must-not-fire variant is quiet"
bound = true
+++

Fixture lints/q003-named-kind-package: kind Stress = Pressure assigned to Pressure must raise Q003 (lints.md 3). found while working ~T71T26K
