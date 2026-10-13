+++
id = "01M4SPWNEF5D7VN10KTWKPM465"
title = "Affine temperature units and TemperatureDelta"
type = "story"
category = "in-progress"
priority = "medium"
points = 3
parent = "01M4SPWFZP4041AN6YQE08X2AN"
reporter = "lognd"
created = "2026-10-13T04:03:51Z"
updated = "2026-10-13T06:15:30Z"
persona = "design engineer"
capability = "use K, degC and degF with exact offsets"
outcome_text = "temperature differences never silently use absolute scales"
labels = ["creates:crates/cadetta-units/src/affine.rs"]
scope = ["crates/cadetta-units/src/affine.rs"]

[[acceptance]]
text = "Given two Temperatures, when subtracted, then the result is a TemperatureDelta (quantities.md 3)"
bound = true

[[acceptance]]
text = "Given an affine unit (degC, degF), when multiplied, then an error value is returned (quantities.md 3)"
bound = true

[[acceptance]]
text = "Given degF and degC literals, when converted to K, then the exact offset is applied rationally (quantities.md 3)"
bound = false

[[acceptance]]
text = "Given dK, ddegC and ddegF, when parsed, then they are TemperatureDelta units with no offset (quantities.md 3)"
bound = false
+++
