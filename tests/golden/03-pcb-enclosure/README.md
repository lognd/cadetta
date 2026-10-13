# 03-pcb-enclosure
A 3D-printed enclosure for a Raspberry Pi 4B (package acme.pienclosure, a single module in src/lib.cdt).
The Pi is a cited ots; Insert is an M2.5 heat-set insert; Fdm is a measured
process with hole, slot, boss and lip compensation rules.
Exercises: datum-typed Point2 arrays (rect_pattern on a pattern datum in the
Pi and in the base), `place pi.holes_site at base.standoff_site`, one
ScrewJoint for the whole pattern with `at`/`onto` and `tap = Vendor`
(geometry.md 3, stdlib.md 3), process designed vs drawn values
(language.md 3.8, quantities.md 5.6), and a fit on the insert hole.
Notable: the insert fit is undecided (C002). Clearance -0.4 +-0.15 gives
[-0.55, -0.25], which straddles the required [-0.4, -0.1]. Checks and fits
read designed values; the `.drawn` values (3.4mm hole, 61.9mm lip) are pinned
separately. Every check holds. Joint thread-depth results depend on the
unspecified IsoDefault policy, so they are not pinned.
