+++
id = "01M4SPWNBYWT610BHC6GNSP35P"
title = "Built-in unit table with exact SI factors"
type = "story"
category = "in-progress"
priority = "medium"
points = 5
parent = "01M4SPWFZP4041AN6YQE08X2AN"
reporter = "lognd"
created = "2026-10-13T04:03:51Z"
updated = "2026-10-13T06:16:20Z"
persona = "design engineer"
capability = "use every unit in the quantities.md table (m mm in ft deg N*m psi rpm ...)"
outcome_text = "I can author values in the units my datasheets use"
labels = ["creates:crates/cadetta-units/src/units.rs"]
scope = ["crates/cadetta-units/src/units.rs"]

[[links]]
kind = "blocks"
target = "01M4SPWNEF5D7VN10KTWKPM465"

[[links]]
kind = "blocks"
target = "01M4SPWNHPTRETQBYCM6EA2GEV"

[[links]]
kind = "blocks"
target = "01M4SPWT6N1HY3MHN544GZJYS9"

[[links]]
kind = "blocks"
target = "01M4ST7A6D0NTQ7XV9RDV6RBGM"

[[acceptance]]
text = "Given unit `in`, when looked up, then the factor is exactly 127/5000 (quantities.md 3)"
bound = true

[[acceptance]]
text = "Given every unit in the quantities.md 3 table, when looked up, then it has an exact factor r*pi^k and a default kind"
bound = false

[[acceptance]]
text = "Given `N*m*s`, when parsed, then it yields an anonymous kind of dimension M L^2 T^-1 (quantities.md 4.1 rule 1)"
bound = false

[[acceptance]]
text = """Given `5"` and `2'`, when resolved, then they alias in and ft (quantities.md 2)"""
bound = false

[[acceptance]]
text = "Given `%`, when resolved, then 5% is exactly 1/20 of kind Ratio (quantities.md 2)"
bound = false

[[acceptance]]
text = "Given a user declaration `unit kip = 1000lbf`, when registered, then it resolves like a built-in (language.md 3.10)"
bound = false
+++
