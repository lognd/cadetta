+++
id = "01M4SPWNHPTRETQBYCM6EA2GEV"
title = "Kinds, kind algebra table and Count/Ratio rules"
type = "story"
category = "in-progress"
priority = "medium"
points = 8
parent = "01M4SPWFZP4041AN6YQE08X2AN"
reporter = "lognd"
created = "2026-10-13T04:03:51Z"
updated = "2026-10-13T06:20:07Z"
persona = "design engineer"
capability = "have kinds such as Torque, Energy, Angle, Ratio refine dimensions"
outcome_text = "a Torque is never silently assigned to an Energy"
labels = ["creates:crates/cadetta-units/src/kind.rs"]
scope = ["crates/cadetta-units/src/kind.rs", "crates/cadetta-units/src/dimension.rs", "crates/cadetta-units/src/lib.rs", "docs/spec/quantities.md"]

[[links]]
kind = "blocks"
target = "01M4SPWNNDNT60Z38K24AEKVAE"

[[links]]
kind = "blocks"
target = "01M4SPWNVX14PTZ4B19436P66Q"

[[links]]
kind = "blocks"
target = "01M4SPWPKGXD3TJ78NPJH8Q9VZ"

[[acceptance]]
text = "Given each row of the kind algebra table, when the operands are multiplied or divided, then the named kind results (quantities.md 4.1 table)"
bound = true

[[acceptance]]
text = "Given a named kind scaled by a dimensionless value (`thickness / 2`), when evaluated, then the kind is kept (quantities.md 4.1 rule 3)"
bound = true

[[acceptance]]
text = "Given Force * Length, when multiplied, then the kind is anonymous, deliberately not Torque or Energy (quantities.md 4.1 rule 3)"
bound = false

[[acceptance]]
text = "Given `sqrt(Area)` and `cbrt(Volume)`, when evaluated, then the kind is Length; given `sqrt(Length)` then Q011 root-of-dimension is returned (quantities.md 4.1, Q011)"
bound = false

[[acceptance]]
text = "Given a Torque assigned to an Energy parameter without `as`, when checked, then Q003 is signalled; `x as Energy` is accepted (quantities.md 4.1 rules 4-5)"
bound = false

[[acceptance]]
text = "Given an anonymous kind of equal dimension and angle, when assigned to a named kind, then it is accepted (quantities.md 4.1 rule 4)"
bound = false

[[acceptance]]
text = "Given a non-integer Count, when constructed, then Q009 is signalled; floor/ceil/round of a dimensionless value yield a Count, ties away from zero (quantities.md 4.2)"
bound = false

[[acceptance]]
text = "Given a Count assigned to a Ratio, when checked, then it is accepted; the reverse is rejected (quantities.md 4.1 rule 4)"
bound = false

[[acceptance]]
text = "Given `kind Stress = Pressure` and `kind HeatFlux = Power / Area`, when registered, then they are distinct named kinds of the same dimension (language.md 3.10)"
bound = false

[[acceptance]]
text = "Given the five angle worked cases (1500rpm*8mm/rev to Velocity clean; 1500rpm*8mm Q008; torque over lead clean; torque over a Length lead Q008; 50N*9mm to Torque clean by the lever-arm exception), when checked, then each yields the stated diagnostic (quantities.md 4.3)"
bound = false
+++
