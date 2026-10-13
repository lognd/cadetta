# 02 bearing housing

Two SKF 608-2RS1 bearings on a stepped shaft in a through-bored housing,
a spacer ring between the outer rings and a DIN 472 circlip at one end.
Housing bore is H7 on the bearing OD, shaft seat is j5 on the bearing
bore, both one-sided `fit`s against vendor tolerances.

Layout: package `acme.bearing_housing`; `src/lib.cdt` holds the design,
`src/vendor.cdt` is the module `vendor` with `pub` `Skf608` and
`Din472Circlip22` (formerly namespace `acme.vendor`).

Exercises: one-sided fits and roles (language.md 3.7), `ots` with
`std::commercial::Sourced` and cited bounds (stdlib.md 6), DIN 472 table
lookups with their tolerance classes (stdlib.md 2), a non-parameterized
`Layout` read by name as one shared variable (language.md 3.1), analysis
`let`s with affine worst-case tolerance (quantities.md 5.3, 5.4), `for`
checks over a part array, `satisfies` against vendor bounds, and `coaxial`
between two shaft-seat datums (geometry.md 2, 5.1).

Notable: `axial_play_ok` is undecided by design (play in [0, 0.64] mm vs
a 0.5 mm limit; the lower bound is exactly 0 and still holds). Of the
two fit checks, `seat_creep` and `seat_squeeze` are also undecided (j5
may be loose or tight). Ten verdicts, three C002 warnings, no errors.
