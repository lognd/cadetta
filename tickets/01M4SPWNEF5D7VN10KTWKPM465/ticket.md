+++
id = "01M4SPWNEF5D7VN10KTWKPM465"
title = "Affine temperature units and TemperatureDelta"
type = "story"
category = "todo"
priority = "medium"
points = 3
parent = "01M4SPWFZP4041AN6YQE08X2AN"
reporter = "lognd"
created = "2026-10-13T04:03:51Z"
updated = "2026-10-13T04:03:51Z"
persona = "design engineer"
capability = "use K, degC and degF with exact offsets"
outcome_text = "temperature differences never silently use absolute scales"
labels = ["creates:crates/cadetta-units/src/affine.rs"]
scope = ["crates/cadetta-units/src/affine.rs"]

[[acceptance]]
text = "Given two Temperatures, when subtracted, then the result is a TemperatureDelta"
bound = false

[[acceptance]]
text = "Given an affine unit, when multiplied, then an error is returned"
bound = false
+++
