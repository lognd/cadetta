+++
id = "01M4V2K8M28G505PDWGFSJCX9G"
title = "E010 role attribute not honoured for mmc/lmc"
type = "bug"
category = "done"
outcome = "done"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T16:47:40Z"
updated = "2026-10-15T02:20:16Z"
scope = ["crates/cadetta-lint/**", "crates/cadetta-cli/tests/c004_e010.rs", "tests/conformance/known-gaps.toml", "changelog.d/01M4V2K8M28G505PDWGFSJCX9G.fixed.md"]

[[acceptance]]
text = "#[role(hole|shaft)] suppresses E010 on .mmc/.lmc; absent role still raises it"
bound = true
+++

Fixture lints/e010-mmc-without-role: #[role(hole)] still raises E010. Spec text fixed to hole or shaft (quantities.md 5.6). found while working ~T71T26K
