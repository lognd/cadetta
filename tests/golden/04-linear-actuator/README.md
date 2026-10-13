# 04 linear actuator

A 300 mm linear actuator: NEMA 17 stepper, flex coupler, Tr8x8 (P2, 4-start) lead screw with a
brass nut, twin MGN12 rails and carriages, a 608 floating-end bearing, and three made parts
(motor mount, end block, carriage plate). It is one package, `acme.actuator`, with modules: `src/lib.cdt` (the
assembly), `drive` and `structure` (`pub` models), and `parts::bearings` and
`parts::motion` (`pub` vendor items; `parts.cdt` is absent). Canonical paths of
instances are unchanged (packages.md 2, 3).

It exercises: parameterized ots families with class and instance access (language.md 3.2),
sum/max roll-ups over `std::commercial::Sourced` members (stdlib.md 6), ISO 286 fits with required
ranges and a waiver (language.md 3.7, lints.md 2.1), angle-exponent quantities in the drivetrain
let-analysis (quantities.md 4.3), and one `ScrewJoint` for the motor's four-hole pattern with
`at`/`onto`, `tap = Vendor` and `locating = true` (stdlib.md 3, geometry.md 3 to 5).

Notable: the journal fit overlaps its required range only in part, so it is C002 (not F004). The
waiver on it has no lock record, so W002 fires and the C002 reappears. Drivetrain angles and thrust
are inexact and pinned to the digits shown. The joint's thread_depth check is not pinned because
it depends on the std default policy's bottom clearance.
