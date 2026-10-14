+++
id = "01M4TA3C37X4GWR2YBJQ1ZBN9Y"
title = "Component fits not derived for lowering"
type = "bug"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T09:39:34Z"
updated = "2026-10-14T22:14:02Z"
scope = ["crates/cadetta-ir", "crates/cadetta-sema/src/fitderive.rs", "crates/cadetta-sema/src/lib.rs", "crates/cadetta-cli/src/pipeline.rs", "crates/cadetta-cli/tests/derived_fits.rs", "tests/conformance/known-gaps.toml", "docs/spec/**", "changelog.d/**"]

[[acceptance]]
text = "A component fit declaration (including its [lo .. hi] required range) is evaluated by the pipeline and appears in the lowered IR Component.fits with its clearance, fit_type and required bound"
bound = true

[[acceptance]]
text = "F004 fires through the pipeline (analyze / cadetta check) when a fit clearance lies entirely outside its required range, and does not fire when the range is satisfied"
bound = true
+++

The pipeline called lower with empty fits. Derive each component fit (clearance, fit type, required range) so Component.fits is populated and F004 fires through cadetta check. The variant half is split out to ~JMCB8NQ (evaluator bug reading through part slots) and ~H360TSR (variant evaluations).
