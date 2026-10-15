+++
id = "01M50ESVA3KPJ0WPQ4GR8GYSJR"
title = "Conformance: must-not-fire value expectations in solvers/output-decimal-exact and role-output-value-is-recorded contradict their own .out.json"
type = "bug"
category = "todo"
priority = "medium"
points = 1
reporter = "lognd"
created = "2026-10-15T18:57:11Z"
updated = "2026-10-15T18:59:01Z"
scope = ["tests/conformance/solvers/**", "tests/conformance/known-gaps.toml"]
+++

Found while working ~K4JKH27. The must_not_fire value rows repeat the must_fire nominal (1/10mm; 3.3mm) but the must-not-fire fixtures record 0.3mm and 2mm, and the harness requires equality. Fix the expect.json rows to the recorded values (3/10mm; 2mm) and drop the two known-gaps entries.
