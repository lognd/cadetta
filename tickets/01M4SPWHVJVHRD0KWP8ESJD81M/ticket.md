+++
id = "01M4SPWHVJVHRD0KWP8ESJD81M"
title = "Exact rational magnitudes and SI dimension vectors"
type = "story"
category = "todo"
priority = "medium"
points = 3
parent = "01M4SPWFZP4041AN6YQE08X2AN"
reporter = "lognd"
created = "2026-10-13T04:03:47Z"
updated = "2026-10-13T04:03:56Z"
persona = "cadetta contributor"
capability = "represent a magnitude as an arbitrary-precision rational over a 7-element SI dimension vector"
outcome_text = "values are exact and comparable only across equal dimensions"
labels = ["creates:crates/cadetta-units/src/{rational,dimension}.rs"]
scope = ["crates/cadetta-units/src/{rational,dimension}.rs"]

[[links]]
kind = "blocks"
target = "01M4SPWNBYWT610BHC6GNSP35P"

[[acceptance]]
text = "Given `1in`, when stored, then its magnitude is exactly 127/5000 m"
bound = false

[[acceptance]]
text = "Given lengths and masses, when added, then a dimension mismatch is returned as an error value"
bound = false
+++
