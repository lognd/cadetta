+++
id = "01M510QYB5KCDF6BBB893GZBM2"
title = "Emit fit .hole/.shaft/.clearance as IR member values (goldens 02, Bracket)"
type = "task"
category = "in-progress"
priority = "medium"
points = 3
reporter = "lognd"
created = "2026-10-16T00:10:43Z"
updated = "2026-10-16T05:16:35Z"
idempotency_key = "fit-member-values"
scope = ["crates/cadetta-sema/src/**"]

[[acceptance]]
text = "tests/golden/02-bearing-housing no longer reports fit paths missing from the IR"
bound = true
+++

After ~VWWVNS4 fits evaluate per instance, but goldens still report bore_fit.hole/shaft/clearance, bearing_seat.* and Bracket.tube_seat.hole as 'path not in the IR': fits are not emitted as member values. Likely needs an IR-shape decision (see ~3YZ3NSG).
