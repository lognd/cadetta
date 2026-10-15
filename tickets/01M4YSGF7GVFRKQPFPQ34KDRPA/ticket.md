+++
id = "01M4YSGF7GVFRKQPFPQ34KDRPA"
title = "Materials index and enum compare in a check not evaluated (materials-tap-class C001)"
type = "bug"
category = "in-progress"
priority = "medium"
points = 2
reporter = "lognd"
created = "2026-10-15T03:25:49Z"
updated = "2026-10-15T12:30:55Z"
scope = ["crates/cadetta-sema/**"]

[[acceptance]]
text = "A check comparing enum variants (e.g. a Materials table cell == TapClass::Hard) decides == and != instead of EVAL"
bound = true

[[acceptance]]
text = "Ordering enum variants is an EVAL error"
bound = true
+++

found while working ~6K58FR1. Fixture stdlib/materials-tap-class expects C001 at acme.demo::P.steel_hard; both sides give EVAL a quantity is required here for Materials[Material::X].tap_class == TapClass::Hard.
