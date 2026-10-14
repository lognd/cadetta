+++
id = "01M4TA3C37X4GWR2YBJQ1ZBN9Y"
title = "Fits and variant evaluations not derived for lowering"
type = "bug"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T09:39:34Z"
updated = "2026-10-14T22:08:44Z"
scope = ["crates/cadetta-ir", "crates/cadetta-sema/src/fitderive.rs", "crates/cadetta-sema/src/lib.rs", "crates/cadetta-cli/src/pipeline.rs", "crates/cadetta-cli/tests/derived_fits.rs", "tests/conformance/known-gaps.toml", "docs/spec/**", "changelog.d/**"]

[[acceptance]]
text = "A component fit declaration (including its [lo .. hi] required range) is evaluated by the pipeline and appears in the lowered IR Component.fits with its clearance, fit_type and required bound"
bound = true

[[acceptance]]
text = "F004 fires through the pipeline (analyze / cadetta check) when a fit clearance lies entirely outside its required range, and does not fire when the range is satisfied"
bound = true
+++

found while working ~E631Y52. pipeline calls lower with empty variants and fits.
