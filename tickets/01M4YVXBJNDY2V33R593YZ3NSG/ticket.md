+++
id = "01M4YVXBJNDY2V33R593YZ3NSG"
title = "M003 not raised: IR Fit has no real hole/shaft operand paths"
type = "bug"
category = "in-progress"
priority = "medium"
points = 5
reporter = "lognd"
created = "2026-10-15T04:07:49Z"
updated = "2026-10-16T04:31:58Z"
scope = ["crates/cadetta-ir/src/**", "crates/cadetta-sema/src/fitderive.rs", "crates/cadetta-sema/src/lower.rs", "tests/conformance/known-gaps.toml", "docs/spec/ir.md", "docs/schemas/cadetta.ir.1.json", "crates/cadetta-lint/src/rules/m.rs"]

[[acceptance]]
text = "Fit.hole/shaft carry the real operand member paths (fitderive tests)"
bound = true

[[acceptance]]
text = "M003 fires at the real fit operand (conformance stdlib/m003-hole-fit-side, gap removed)"
bound = true
+++

found while working ~PGR0M75. fitderive.rs derive_one sets Fit.hole/shaft to synthetic <fit>.hole/.shaft paths, so check_m003 never matches the real operand (j.clearance_hole in stdlib/m003-hole-fit-side, expected M003 at acme.demo::Box/j.clearance_hole). Needs an IR decision: carry the operand paths in ir::Fit (schema/digest/decode/json) or change what hole/shaft mean.
