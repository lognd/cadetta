+++
id = "01M4V1HE1BBZJAMC8BM0YYT3TP"
title = "Quantities: 20degC - 5degC yields Temperature not TemperatureDelta; affine multiply not rejected"
type = "bug"
category = "done"
outcome = "done"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T16:29:12Z"
updated = "2026-10-15T02:08:31Z"
idempotency_key = "Z48-affine"
scope = ["crates/cadetta-sema/**", "changelog.d/01M4V1HE1BBZJAMC8BM0YYT3TP.fixed.md", "tests/conformance/known-gaps.toml"]

[[acceptance]]
text = "Affine unit multiply or divide (20degC * 2) is Q002"
bound = true

[[acceptance]]
text = "20degC - 5degC is a TemperatureDelta, not a Temperature"
bound = true
+++

found while working ~Z48GXKX. quantities.md 3 (spec sentence added: affine multiply/divide is Q002). Fixture q002-affine-multiply fails; q003-temperature-delta fixture passes today only by accident of the Q003, see the gap.
