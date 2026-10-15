+++
id = "01M4V1HDJS2MDQF8GRK4QX9B1C"
title = "Quantities: duplicate cascading diagnostics (E001, Q001/Q002, EVAL+Q002)"
type = "bug"
category = "done"
outcome = "done"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T16:29:11Z"
updated = "2026-10-15T02:17:01Z"
idempotency_key = "Z48-cascade"
scope = ["crates/cadetta-sema/**", "crates/cadetta-syntax/src/parser.rs", "crates/cadetta-cli/src/pipeline.rs", "crates/cadetta-cli/tests/quantity_diag_once.rs", "tests/conformance/**", "docs/spec/quantities.md", "changelog.d/**"]

[[acceptance]]
text = "Each E001 lexical error, Q001/Q002/Q008 and E002 quantity fault is reported exactly once (conformance quantities e001-*, e002-unit, q001-*, q002-*, q008-* pass)"
bound = true

[[acceptance]]
text = "check and require conditions are kind-checked and carry a member path (q001-compare, q008-compare)"
bound = true
+++

found while working ~Z48GXKX. A single fault emits two diagnostics: E001 lex errors emit a second E001 (e001-*); '5mm + 3' Q001+Q002 (q001-operand, q001-tolerance*, q001-trig); '5mm ^ 1.5' and 'round_to(2.1mm,1s)' EVAL+Q002 (q002-power-noninteger, q002-round-step); sin(5mm) two Q002 (q002-trig); unknown unit E002+EVAL (e002-unit); sin(5deg*1deg) Q008+Q002 (q008-trig-exponent); 'x >= 5' in a check gives Q002 not Q001 (q001-compare). Spec: quantities.md 4.4, closed diagnostics list.
