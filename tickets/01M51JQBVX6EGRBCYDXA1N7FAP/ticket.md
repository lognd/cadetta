+++
id = "01M51JQBVX6EGRBCYDXA1N7FAP"
title = "Golden 02: per-instance member paths (BearingUnit/housing.*) missing from IR"
type = "bug"
category = "in-progress"
priority = "medium"
points = 3
reporter = "lognd"
created = "2026-10-16T05:24:58Z"
updated = "2026-10-16T09:04:20Z"
scope = ["crates/cadetta-sema/src/**", "docs/spec/language.md"]
+++

found while working ~93GZBM2: golden reports BearingUnit/housing.bore_fit.* and other instance member paths as not in the IR; also Bracket.bore_fits_tube check not in the IR.
