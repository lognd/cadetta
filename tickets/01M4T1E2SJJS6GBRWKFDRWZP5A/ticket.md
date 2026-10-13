+++
id = "01M4T1E2SJJS6GBRWKFDRWZP5A"
title = "cadetta-fits IT1..IT4 rows wrong for sizes over 30 mm"
type = "bug"
category = "in-progress"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T07:08:07Z"
updated = "2026-10-13T07:12:05Z"
scope = ["crates/cadetta-fits/src/tables.rs"]

[[acceptance]]
text = "Given a size over 30 mm, when IT1..IT4 are looked up, then widths match ISO 286-1 table 1"
bound = false
+++

found while working ~E8NJG6J: IT_TENTHS_UM rows IT1-IT4 are wrong from the 30-50 mm range up (e.g. IT3 30-50 is 4um in ISO 286-1 table 1, table has 5; IT4 30-50 is 7um, has 8). Correct rows are in std/src/fits.cdt ItGrade. IT5..IT18 verified correct.
