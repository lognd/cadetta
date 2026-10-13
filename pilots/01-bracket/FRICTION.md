# Pilot 01: machined L-bracket (friction report)

## 1. Summary

The core quantity model (exact rationals, tolerance intervals, three-valued
checks) handled the numeric side well: edge-distance and engagement checks
read naturally and tolerance propagation gave correct holds/violates
answers. It breaks at the edges of "OTS data": standards tables (ISO 273,
ISO 4762) have no first-class form, tolerances leak into derived
layout/feature dimensions, and non-quantity data (material) has nowhere
to live. About half the design is workaround or `PROPOSED`.

## 2. Findings

F1 (awkward) - language.md 3.1, 3.2. Wanted: ISO 273 clearance hole from
a thread size ("normal fit per ISO 273"). Spec: no standards-table lookup;
only ISO 286 `fit`. I hand-copied an `ots Iso273` with one param per size,
and `mount_hole_dia = Iso273.medium_m5` is a hard-coded row nothing ties
to the screw. Swap the screw to M6 and the hole silently stays 5.5.
Suggest: `hole mount_hole = clearance medium for Iso4762M5x25.thread_dia`
resolving through `std.clearance.iso273`; ship ISO 4762 heads likewise.

F2 (awkward) - quantities.md 5.1, 5.2. Wanted: use a toleranced value as a
nominal for sizing. `mount_cb_dia = head_dia + 1mm` inherits
`+0 -0.22` and `clamp_bolt_offset` inherits the bore's `+0.13/0`; the IR
now claims those tolerances on counterbore and bolt location, and
M003/C002 reasoning uses them. Suggest `nominal(x)`, `max_material(x)`,
`lmc(x)` functions returning untoleranced values.

F3 (awkward) - quantities.md 5. Wanted: "maximum only" dimensions (ISO
4762 gives dk max/min). Spec forces `8.5mm +0mm -0.22mm`, abusing nominal
=max. The limits form `8.28mm..8.5mm` would put nominal at the midpoint
8.39 and the default `emit point = nominal` would size the counterbore
wrongly. Suggest `8.5mm max` / `8.28mm min` one-sided forms.

F4 (awkward) - language.md 3.1, OQ-4. ISO 4762 is one family with many
lengths, but ots cannot be parameterised and instances cannot override.
I needed `Iso4762M5x25` and `Iso4762M4x20`; an M5x16 variant would
copy head dims and trip D001. Suggest `ots Iso4762M5 { length: Length }`
with per-`part` args: `part screw_a: Iso4762M5(length = 25mm)`.

F5 (nit) - language.md 4 (N001). Identifiers cannot be `Iso4762_M5`;
PascalCase with digits (`Iso4762M5x25`) is unreadable. Say whether
digit/underscore runs are legal in component names.

F6 (awkward) - language.md 3, 3.3. Material designation (`"6061-T6"`),
finish, thread class: strings are not values. Only doc comments carry
them (dropped from output). Suggest string-valued params:
`designation: Text = "6061-T6"`.

F7 (awkward) - quantities.md 2 vs language.md 6. `bracket.count == 2` is
a bare literal on the check rhs: not a "factor or exponent", so Q001 by
the letter. I added an assembly param `required_screws: Count = 2` as a
workaround. Also, no way to derive a count from `part` declarations
(two `screw_*` parts vs `mount_hole_count` are unrelated). Suggest
allowing bare numbers beside Count/Ratio operands, and `count(screw_*)`.

F8 (nit) - language.md 3 grammar vs 3.1. `check` is in `member`, but only
assemblies are said to declare checks. I used checks inside `model`.
Clarify.

F9 (awkward) - quantities.md 5.1 and tube tolerance. Commercial tube OD
tolerance is not ISO 286; I assumed `h11` so the fit F003 would agree.
A vendor `+- 0.15mm` OD would conflict with `fit ... on Tube.od` (F003)
with no way to express a non-ISO "shaft side". Suggest `fit` accept
explicit limits: `fit seat = H11/(20mm +- 0.15mm) on ...`.

F10 (nit) - quantities.md 3. Spelling of `g/cm^3` units fine, but there
is no `Stress` kind (OQ-3): `yield_strength = 240MPa` is a Pressure.

## 3. Hypothetical `cadetta check` output

- S001 on none: every ots has component-level `source`. Correct, quiet.
- D001 (info) on `Iso4762M4x20.thread_dia`/`hex_key` vs other 4mm/3mm
  values: noise (common values list helps; M4 vs `tap_drill` coincidences).
- M003 (warning) on `Bracket.mount_hole_dia`, `thickness`,
  `clamp_cap_depth` etc. referenced by assembly checks without a
  tolerance: noise; machining tolerance is a drawing-note matter here.
- M002 (info) on `1.5 *` in assembly checks: spec says simple fractions
  are exempt, so likely silent; ambiguous whether 1.5 qualifies.
- N003: none (`dia` is not a unit). N002: none.
- Q001 (error) on `== 2`? by the letter of the spec: wrong (see F7), I
  avoided it.
- C001/C002: none; all checks `holds` (below).

Uncaught real mistakes: wrong ISO 273 row for the screw (F1); M5 hole
count vs number of `part` screws; clamp bolt tapped depth (tapped thread
length) and counterbore overlap with the fillet in the shelf leg are not
modelled; head-clearance diameters whose nominal is the max rely on a
convention nothing checks (F3); hole position tolerance (GD&T) absent.

## 4. Evaluation trace

(a) `clamp_tap_drill = 4mm - 0.7mm`: 4mm = 1/250 m = 40/10000;
0.7mm = 7/10000; result 33/10000 m = 3.3mm, exact, no tolerance, kind
Length (both operands Length). Clean.

(b) `mount_cb_dia = 8.5mm +0mm -0.22mm + 1mm`: nominal 17/2000 + 1/1000 =
19/2000 m (9.5mm); lower dev -0.22mm = -11/50000 m, upper 0, so limits
[9.28, 9.5]mm = [29/3125 m, 19/2000 m]. Ambiguity: ir.md `Value.lower/upper`
do not say whether they are deviations or absolute limits.

(c) `edge_distance = (width - mount_spacing)/2` with width 56 +-0.1, spacing
32 +-0.1: worst-case `-` adds deviations: 24 +-0.2 mm; `/2` scales: 12
+-0.1 -> [11.9, 12.1]mm. Check `>= 2 * 5.5mm = 11mm`: lower 11.9 >= 11,
so `holds`. Ambiguity: whether `-` of a +-0.1 minuend and subtrahend
yields +-0.2 (yes per 5.1) and whether `Count`-valued `2 *` keeps the
Length kind (yes, anonymous scalar factor).

Also ambiguous: `fit tube_seat ... on Tube.od` uses the OD's own
`20mm h11` as the shaft side (F003 agrees) but the spec does not say
whether `tube_seat.hole` nominal is 20mm (so `clamp_bore / 2` nominal
10mm, upper +0.065mm). I assumed that.
