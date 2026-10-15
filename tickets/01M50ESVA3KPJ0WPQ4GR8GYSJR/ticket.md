+++
id = "01M50ESVA3KPJ0WPQ4GR8GYSJR"
title = "Conformance: must-not-fire value expectations in solvers/output-decimal-exact and role-output-value-is-recorded contradict their own .out.json"
type = "bug"
category = "in-progress"
priority = "medium"
points = 1
reporter = "lognd"
created = "2026-10-15T18:57:11Z"
updated = "2026-10-15T19:06:09Z"
scope = ["tests/conformance/solvers/**", "tests/conformance/known-gaps.toml", "docs/spec/solvers.md"]

[[acceptance]]
text = "must_not_fire value rows match the recorded outputs and the two known-gaps entries are removed (conformance passes on goway)"
bound = true

[[acceptance]]
text = "docs/spec/solvers.md notes outputs are evaluated from the recorded .out.json"
bound = true
+++

Found while working ~K4JKH27. The must_not_fire value rows repeat the must_fire nominal (1/10mm; 3.3mm) but the must-not-fire fixtures record 0.3mm and 2mm, and the harness requires equality. Fix the expect.json rows to the recorded values (3/10mm; 2mm) and drop the two known-gaps entries.
