+++
id = "01M50J83ATKTE83FQVVG0V5V7D"
title = "cadetta conformance: library behaviour for driven dimensions, Q001 kg-vs-mm and SWFN(1) unparsable read fixtures"
type = "task"
category = "in-progress"
priority = "medium"
points = 3
reporter = "lognd"
created = "2026-10-15T19:57:24Z"
updated = "2026-10-15T21:56:20Z"
scope = ["crates/cadetta-adapter/**", "crates/cadetta-cli/tests/**", "tests/conformance/known-gaps.toml", "docs/spec/emitters/sw-equations.md"]

[[acceptance]]
text = "A driven dimension reads exact:false with a reason"
bound = true

[[acceptance]]
text = "A unit of another dimension than the locator's native unit reads inexact, never Q001"
bound = true

[[acceptance]]
text = "A call to an unknown equation function reads as an unparsed raw line"
bound = true
+++

split from ~0MC1ZA9: adapters/read-driven-dimension-inexact (library has no driven-dimension concept), adapters/read-never-raises-q001 (dimension-mismatch inexactness kg vs mm), adapters/read-unparsable-expression-inexact (equations parser accepts SWFN(1) as a call). Remove each known-gaps.toml entry when done.
