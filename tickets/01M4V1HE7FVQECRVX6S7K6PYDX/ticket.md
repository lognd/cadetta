+++
id = "01M4V1HE7FVQECRVX6S7K6PYDX"
title = "Quantities: pi-factor floor Q006, compare Q008, 1deg+1rad EVAL"
type = "bug"
category = "done"
outcome = "done"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T16:29:12Z"
updated = "2026-10-15T02:23:07Z"
idempotency_key = "Z48-angle"
labels = ["creates:changelog.d/01M4V1HE7FVQECRVX6S7K6PYDX.fixed.md"]
scope = ["crates/cadetta-sema/**", "crates/cadetta-units/src/inexact.rs", "crates/cadetta-cli/tests/quantity_pi_inexact.rs", "docs/spec/quantities.md", "tests/conformance/known-gaps.toml", "changelog.d/01M4V1HE7FVQECRVX6S7K6PYDX.fixed.md"]

[[acceptance]]
text = "floor of a pi-carrying value is inexact with a Q006 root; a pi-free ratio floors clean"
bound = true

[[acceptance]]
text = "1deg + 1rad is an inexact value (Q006), not an EVAL error; exact-zero-normalised sums stay clean"
bound = true

[[acceptance]]
text = "Q008 fires for rpm*length compared to a speed and not for the per-rev form"
bound = true
+++

found while working ~Z48GXKX. quantities.md 1, 4.1 rule 6, 4.3. Fixtures q006-count-pi, q008-compare, exact-zero-normalised (1deg+1rad gives EVAL; spec 1 says inexact result with Q006; spec 5.4 only errors in affine forms, mild spec inconsistency to settle).
