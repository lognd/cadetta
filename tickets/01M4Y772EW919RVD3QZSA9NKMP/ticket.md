+++
id = "01M4Y772EW919RVD3QZSA9NKMP"
title = "Surface fits::analyze failures (F001/F002/F003/C002) as user diagnostics from the pipeline"
type = "task"
category = "todo"
priority = "medium"
points = 3
reporter = "lognd"
created = "2026-10-14T22:06:07Z"
updated = "2026-10-15T09:43:06Z"
scope = ["crates/cadetta-sema/src/fitderive.rs", "crates/cadetta-cli/src/pipeline.rs", "docs/spec/lints.md", "tests/conformance/known-gaps.toml"]

[[acceptance]]
text = "A fit that fails fits::analyze (e.g. F003 tolerance conflict, F001/F002 bad class) produces the catalog diagnostic through analyze/cadetta check, with a conformance fixture leaving known-gaps.toml"
bound = false
+++

Split from ~Q1ZBN9Y. fitderive::derive_fits only logs (warn) a FitFailure and omits the fit; the diagnostics never reach the user. Fixtures still in tests/conformance/known-gaps.toml: language/f003-fit-tolerance-conflict, language/fit-clearance-member, and the other 'table/fit/variant pipeline not implemented yet' entries.
