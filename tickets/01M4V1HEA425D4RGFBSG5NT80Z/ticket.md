+++
id = "01M4V1HEA425D4RGFBSG5NT80Z"
title = "Quantities: inexact operand in check gives EVAL, spec 5.8 wants undecided/holds with 1e-9 widening"
type = "bug"
category = "done"
outcome = "done"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T16:29:12Z"
updated = "2026-10-15T02:23:07Z"
idempotency_key = "Z48-inexactcheck"
labels = ["creates:changelog.d/01M4V1HEA425D4RGFBSG5NT80Z.fixed.md"]
scope = ["crates/cadetta-lint/**", "crates/cadetta-units/src/inexact.rs", "crates/cadetta-sema/**", "crates/cadetta-cli/tests/quantity_pi_inexact.rs", "docs/spec/quantities.md", "tests/conformance/known-gaps.toml", "changelog.d/01M4V1HEA425D4RGFBSG5NT80Z.fixed.md"]

[[acceptance]]
text = "an inexact operand near the boundary in a check gives C002 (undecided), not EVAL"
bound = true

[[acceptance]]
text = "an inexact operand clear of the boundary holds with no diagnostic"
bound = true
+++

found while working ~Z48GXKX. quantities.md 5.8. Fixture c002-inexact-boundary.
