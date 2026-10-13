+++
id = "01M4SPWHVJVHRD0KWP8ESJD81M"
title = "Exact magnitudes r*pi^k and dimension vectors"
type = "story"
category = "todo"
priority = "medium"
points = 5
parent = "01M4SPWFZP4041AN6YQE08X2AN"
reporter = "lognd"
created = "2026-10-13T04:03:47Z"
updated = "2026-10-13T05:01:21Z"
persona = "cadetta contributor"
capability = "represent a magnitude as an arbitrary-precision rational over a 7-element SI dimension vector"
outcome_text = "values are exact and comparable only across equal dimensions"
labels = ["creates:crates/cadetta-units/src/{rational,dimension}.rs"]
scope = ["crates/cadetta-units/src/{rational,dimension}.rs"]

[[links]]
kind = "blocks"
target = "01M4SPWNBYWT610BHC6GNSP35P"

[[acceptance]]
text = "Given `1in`, when stored, then its magnitude is exactly 127/5000 m with pi exponent 0 (quantities.md 1)"
bound = false

[[acceptance]]
text = "Given `360deg / 1.8deg`, when evaluated, then the result is exactly 200 because pi^k cancels (quantities.md 1)"
bound = false

[[acceptance]]
text = "Given magnitudes with different pi exponents, when added, then the result is inexact unless one is zero (quantities.md 1)"
bound = false

[[acceptance]]
text = "Given lengths and masses, when added, then a dimension mismatch is returned as an error value, not a panic (quantities.md 1)"
bound = false

[[acceptance]]
text = "Given a currency unit such as USD, when its dimension is built, then it is its own base dimension beyond the seven SI ones (quantities.md 1, 3)"
bound = false

[[acceptance]]
text = "Given an angle quantity, when stored, then the integer angle exponent is tracked beside the dimension and does not change the magnitude (quantities.md 1, 4.3)"
bound = false
+++
