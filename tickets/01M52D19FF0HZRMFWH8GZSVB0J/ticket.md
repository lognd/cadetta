+++
id = "01M52D19FF0HZRMFWH8GZSVB0J"
title = "Golden 02-bearing-housing: remaining value/verdict mismatches after EVAL fix"
type = "bug"
category = "todo"
priority = "medium"
points = 3
reporter = "lognd"
created = "2026-10-16T13:04:47Z"
updated = "2026-10-16T13:08:40Z"
scope = ["tests/golden/02-bearing-housing/**"]
+++

found while working ~FCZKA30: run is clean now; 18 mismatches left (groove_dia/width, bore_dia.max, seat_dia.min, groove_side/axial_play max, axial_play_ok undecided vs holds, seats_coaxial conservative, C002 and S008 expectations, bearing[0].radial_clearance.min missing). Triage hand-evaluation vs implementation.
