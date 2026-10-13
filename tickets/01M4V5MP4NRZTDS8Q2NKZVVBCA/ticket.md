+++
id = "01M4V5MP4NRZTDS8Q2NKZVVBCA"
title = "csv.md: note separator and emit-point token are inconsistent"
type = "bug"
category = "in-progress"
priority = "medium"
points = 2
reporter = "lognd"
created = "2026-10-13T17:40:52Z"
updated = "2026-10-13T18:14:31Z"
scope = ["docs/spec/emitters/csv.md", "docs/spec/emitters/json.md", "docs/spec/adapters.md", "tests/conformance/emitters/csv/**"]

[[acceptance]]
text = "Given csv.md 1, json.md 1 and adapters.md 3a, when read, then the note parts are separated by one space and the emit point is written emit=POINT, consistently across the three documents"
bound = true

[[acceptance]]
text = "Given the csv note fixtures, when read, then they assert the exact note text (order, emit= token, fit class) rather than loose substrings"
bound = true
+++

found while working ~9JX3J6F. csv.md 1 says note parts are joined with '; ' but its example row 1 joins literal and citation with a space (1/4in @"ASTM B209"), and row 2 writes 'emit mid' where json.md and sw-equations.md write 'emit=mid'. Decide the separator and token form; then tighten note-order, note-emit-point and note-fit-class fixtures under tests/conformance/emitters/csv/, which assert separator-agnostically today.
