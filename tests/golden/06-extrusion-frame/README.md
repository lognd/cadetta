# 06-extrusion-frame

A 4ft x 2.5ft x 3ft workbench frame in 80/20 1010 extrusion with 1/4-20
hardware, in inches: legs, rails, a diagonal brace, gussets, panel clips and
two plywood shelf panels. Everything is cut to the 1/32in grid with the
vendor's saw tolerance referenced, not restated.

Exercises: fraction, feet-inch and mixed literals (quantities.md 2);
`round_to` with exact results (5.5); exact `atan2` and inexact `sqrt`
(4.3, Q006 as the only diagnostics); Count, Ratio and Mass kind
assignment (4.1); design tolerance on parameters, `total_extrusion_length`
stores a nominal only (5.3); part arrays with `sum` and `count` (language.md
3.5); the four legs are placed on corner points with a Count-range
`place`, and checks read positions back through the placements
(geometry.md 3, 4).

Inch fasteners are not in `std` until 0.2.0 (stdlib.md 1), so the screw and
T-nut are `ots` parts and the gusset engagement is an ordinary check. The
design is clean: every check holds, and the engagement and thread checks are
exact equalities at their limit (3/8in against 1.5 x 1/4in).
