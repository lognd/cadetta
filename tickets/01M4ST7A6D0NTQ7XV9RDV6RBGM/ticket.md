+++
id = "01M4ST7A6D0NTQ7XV9RDV6RBGM"
title = "Display unit selection and fractional formats"
type = "story"
category = "todo"
priority = "medium"
points = 3
parent = "01M4SPWFZP4041AN6YQE08X2AN"
reporter = "lognd"
created = "2026-10-13T05:02:06Z"
updated = "2026-10-13T05:02:06Z"
persona = "design engineer"
capability = "print quantities in the unit I chose, including fractional inches"
idempotency_key = "replan13:Display unit selection and fractional formats"
labels = ["creates:crates/cadetta-units/src/display.rs"]
scope = ["crates/cadetta-units/src/display.rs"]

[[acceptance]]
text = "Given `#[display(unit)]`, `#![display(...)]`, `[units] display` and the authored unit, when a value is printed, then they apply in that order (quantities.md 6)"
bound = false

[[acceptance]]
text = "Given `#[display(in/32)]`, when 45.9375in is printed, then it reads `45-15/16in` (quantities.md 6)"
bound = false

[[acceptance]]
text = """Given `#[display(ft_in)]`, when printed, then it reads `3'9-15/16"` (quantities.md 6)"""
bound = false

[[acceptance]]
text = "Given a money value, when printed, then it uses the currency minor unit and percentages print to one decimal, half away from zero (obligations.md 5.1)"
bound = false
+++
