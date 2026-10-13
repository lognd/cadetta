# Review 3: geometry.md and its integration

Summary
- Development: 3/5. The datum/placement-tree idea is sound and the pilot's tree rule holds, but the flagship example is numerically wrong, ScrewJoint's array form contradicts its pattern arguments, and nothing yet models axes, joint float, or SOLIDWORKS-shaped bindings.
- Single most important fix: make ScrewJoint one joint per pattern (count from `at`) and let its float (clearance slack) enter the placement it implies; today a coaxiality check through bolted parts answers "holds" for loops that can physically fail.
- Counts: 3 blocker, 7 gap, 2 polish.

## Pilot (not in repo)

Layout: base with datum `top`; motor plate (4 x M5 into base); bearing block
(2 x M5 into base); motor (interface, alt has a different mount pattern)
bolted to the plate with 4 x M3 (tap = Vendor); shaft/bore axes along base x
at height 40mm; 4 feet under the base at the corners of a rect_pattern.
Idiom used: each bolted part is placed on a *pattern datum of its host*
(`base.plate_site`), so the nominal position is never restated.

```
use std.fasteners::ScrewJoint

interface Motor {
  datum mount                                  // flange face, z = shaft direction (convention not enforceable, finding 9)
  datum shaft                                  // shaft axis frame, z along shaft
  mount_holes: [Point2 on mount; 4]
  mount_thread: MetricThread
}

ots NemaA: Motor @vendor {
  datum mount = origin
  datum shaft = origin.offset(x = 0mm [+-0.02mm], y = 0mm [+-0.02mm])   // eccentricity
  mount_holes = rect_pattern(2, 2, 31mm, 31mm) [+-0.05mm]
  mount_thread = M3
}
ots NemaB: Motor @vendor {                     // alternate: 26mm square pattern
  datum mount = origin
  datum shaft = origin.offset(x = 0mm [+-0.02mm], y = 0mm [+-0.02mm])
  mount_holes = rect_pattern(2, 2, 26mm, 26mm) [+-0.05mm]
  mount_thread = M3
}

model Base {
  datum top        = origin
  datum plate_site = top.offset(x = 60mm)      // pattern centres are datums
  datum block_site = top.offset(x = 200mm)
  datum underside  = origin.offset(z = -20mm).flip(x)
  plate_holes: [Point2 on plate_site; 4] = rect_pattern(2, 2, 30mm, 40mm) [+-0.05mm]
  block_holes: [Point2 on block_site; 2] = [Point2(0mm, -20mm), Point2(0mm, 20mm)] [+-0.05mm]
  foot_sites:  [Point2 on underside; 4]  = rect_pattern(2, 2, 160mm, 70mm)
  material = Material::Al6061T6
}

model MotorPlate {
  thickness = 10mm
  axis_h    = 40mm [+-0.02mm]
  datum foot       = origin
  datum motor_face = origin.offset(z = axis_h).rotate(y = 90deg)   // z -> +x_base, x -> down
  foot_holes:  [Point2 on foot; 4]       = rect_pattern(2, 2, 30mm, 40mm) [+-0.05mm]
  motor_holes: [Point2 on motor_face; 4] = rect_pattern(2, 2, 31mm, 31mm) [+-0.05mm]
}

model BearingBlock {
  axis_h = 40mm [+-0.02mm]
  datum foot = origin
  datum bore = origin.offset(z = axis_h).rotate(y = 90deg)
  foot_holes: [Point2 on foot; 2] = [Point2(0mm, -20mm), Point2(0mm, 20mm)] [+-0.05mm]
}

model Foot { datum top = origin }

assembly Drive {
  part base:  Base
  part plate: MotorPlate
  part block: BearingBlock
  part motor: Motor = NemaA alt [NemaB]
  part feet[4]: Foot

  place base.origin on origin
  place plate.foot  on base.plate_site
  place block.foot  on base.block_site
  place motor.mount on plate.motor_face
  place feet[i].top on base.underside.offset(x = base.foot_sites[i].x,
                                             y = base.foot_sites[i].y) for i in feet

  // as written in the spec these are 4 / 2 / 4 joint instances each given the WHOLE pattern (finding 1)
  part plate_screws[4]: ScrewJoint(thread = M5, clamp = plate.thickness, host = base,
                                   at = plate.foot_holes, onto = base.plate_holes)
  part block_screws[2]: ScrewJoint(thread = M5, clamp = 10mm, host = base,
                                   at = block.foot_holes, onto = base.block_holes)
  part motor_screws[4]: ScrewJoint(thread = motor.mount_thread, clamp = plate.thickness,
                                   host = motor, tap = Vendor, depth_available = 6mm,
                                   at = plate.motor_holes, onto = motor.mount_holes)

  // coaxial shaft/bore: no axis primitive exists, so two points on the shaft axis
  // carried into the bore frame; the radial offset must be written by hand.
  check coaxial: {
    let p0: Point3 in motor.shaft = Point3(0mm, 0mm, 0mm)
    let p1: Point3 in motor.shaft = Point3(0mm, 0mm, 150mm)   // reach the bearing
    let q0 = p0 in block.bore
    let q1 = p1 in block.bore
    sqrt(q0.x ^ 2 + q0.y ^ 2) <= 0.05mm and sqrt(q1.x ^ 2 + q1.y ^ 2) <= 0.05mm   // `and` is not in the language either
  } "shaft/bore coaxial within 0.05mm"
}
```

### Hand evaluation, primary (NemaA)

Frames (all in mm, base frame T = `base.top`, hole tolerances +-0.05 per
coordinate, each coordinate an independent source variable):

- Joint plate/base: M5 medium clearance 5.5, major 5.0, fixed allowance
  (5.5 - 5.0)/2 = 0.25. Carried into T, plate hole i = plate_site +
  (+-15, +-20) + e_p; base tapped hole i = plate_site + (+-15, +-20) + e_b.
  Per axis the difference e_b - e_p is in [-0.10, 0.10]; distance in
  [0, 0.1414]. 0.1414 <= 0.25: holds, all 4 pairs. (With the spec's +-0.1
  hole tolerances the corner is 0.2828 > 0.25 and the result is a true
  undecided, not a conservative one; a C002 text saying "conservative" would
  mislead.)
- Joint block/base: identical structure, 2 pairs, holds.
- Joint motor/plate: M3 medium clearance 3.4, major 3.0, allowance 0.20;
  placement is identity on `motor_face`; distance in [0, 0.1414]: holds.
- Alternate NemaB: pattern 26 vs 31 square, per-hole offset about 2.5 in
  each axis, distance about 3.54 > 0.20: every value violates, so C001 for
  alt NemaB and I002 (alternate non-viable). Correct and useful; there is no
  way to say "NemaB needs an adapter plate" (finding 9).
- Coaxiality. Shaft origin in T = (60, e_y, h - e_x) (mount x axis is -z_T,
  y is y_T, z is +x_T); bore frame origin = (200, 0, h2), x_B = -z_T,
  y_B = y_T, z_B = +x_T. p0 in bore = (h2 - h + e_x, e_y, -140); p1 = (same,
  same, +10). Radial r = sqrt((h2 - h + e_x)^2 + e_y^2). h, h2, e_x each
  +-0.02: x_B in +-0.06; e_y +-0.02: r in [0, 0.0632]. Against 0.05 it is
  undecided (C002, genuinely, since r = 0 is reachable and 0.0632 is
  reachable). With +-0.01 on all four: x_B +-0.03, r <= 0.0316: holds.
  The two points give the same answer because every rotation is exact; with
  a toleranced tilt p1 would differ from p0 by L*sin(theta).
- What the answer ignores: the plate and block are placed exactly, yet each
  floats up to 0.25 in x and y inside its clearance holes. With float
  `[+-0.25mm]` on the plate and block y offsets, r reaches about 0.55; the
  honest result is "cannot be guaranteed" and a dowel/pilot feature is
  needed. The spec's answer is "holds", or "undecided at 0.063", which is
  too optimistic (finding 3).
- Tree rule: base <- plate <- motor, base <- block, base <- feet[i] is a
  tree rooted at `origin`. The "closed loop through the base" is plate-base-
  block, which closes only in the `coaxial` check, as geometry.md 4 says. It
  holds for this design and forced no awkwardness. It does not remove the
  restatement problem: hole patterns are written on both mating parts (that
  is physical and `aligned` verifies it), but the pattern-datum idiom is
  needed to avoid a third restatement in the offset (finding 2).

## Findings

### 1. blocker: ScrewJoint arrays contradict whole-pattern at/onto (language.md 3.5, geometry.md 1 and 5, stdlib.md 3)

language.md 3.5 says every instance of a part array has the same arguments.
geometry.md 1 and stdlib.md 3 write `part screws[2]: ScrewJoint(..., at =
bracket.holes, onto = rail.tapped)` where `at` is the whole 2-point pattern.
So each of the 2 joint instances checks all 2 pairs (4 pair checks for 2
screws, 16 for the 4-screw case), no instance is tied to "its" hole, and
nothing relates the array count to the pattern count (`screws[3]` with a
2-hole pattern passes). Fix: ScrewJoint is one instance per pattern
(`part screws: ScrewJoint(...)`), its count is `count(at)` (G004 when
`count(onto)` differs), and it owns `part screw[count(at)]` internally so BOM
and mass roll-ups still count one screw per hole. Update the language.md 1
example (`bracket_screws[4]`) accordingly, or, for the no-pattern form, state
that the array count is a free design count and no `at`/`onto` is allowed.

### 2. blocker: the flagship example in geometry.md 1 fails its own check

Rail holes at x = +-25 on `top`; the bracket is placed at
`rail.top.offset(x = 40mm [+-0.3mm])`, so bracket holes land at x = 15 and 65,
40mm away from the tapped holes at -25 and 25. The text then says "the joint
proves both holes line up". Even with offset 0 the +-0.3 placement plus +-0.2
and +-0.1 hole tolerances give 0.6 per axis against the M5 fixed allowance of
0.25, i.e. undecided. Fix: pattern-datum idiom, no offset restated:
`datum site = top.offset(x = 40mm)` on Rail, tapped holes declared `on site`,
`place bracket.base on rail.site`, hole tolerances +-0.05; keep a variant of
the example showing a loosened tolerance flipping to C002. State the idiom
in geometry.md 4: "place a bolted part on a datum of its host that is the
centre of the host's pattern".

### 3. blocker: bolt float never enters a placement, so loop checks are unsound (geometry.md 4 and 5, stdlib.md 3)

geometry.md 4 promises "given every tolerance on the way round", but a part
bolted with clearance holes is placed exactly and the free float (allowance
minus misalignment, up to 0.25mm radial for M5) appears nowhere. `aligned`
passes while the part can sit 0.25mm off, and any chain through it (the
0.05mm coaxiality in the pilot) is decided as if it were dowelled. Fix:
`ScrewJoint` exports `float: Length` (the allowance used by `aligned`) and
`locating: Bool` (default false; `true` when a pilot, dowel or fit locates the
part). Rule: when the clamped part of a joint with `at`/`onto` is placed on
the host's pattern datum, the placement carries an implicit `[+-float]` on x
and y of that edge, one source variable pair per joint, unless
`locating = true`. Add a lint (warning) "bolted placement without locating
feature used in a check tighter than the float". Dowel pins can stay out of
0.1.0 std, but the escape hatch (`locating = true` plus a `fit`) must exist
or the feature is useless for any sub-0.25mm check, which is the stated use.

### 4. gap: no axis, coaxiality, parallelism or angle primitive; radial distance is not safely expressible (geometry.md 2 and 5, language.md 3.8, quantities.md 5.4)

Everything is points. Coaxiality must be hand-built from two points and
`sqrt(x^2 + y^2)`; quantities.md 5.4 says nonlinear products use interval
bounds, so `x ^ 2` of a toleranced x with sign-straddling interval can
produce a negative lower bound and then `sqrt` fails or loosens. There is no
`angle_between(datum, datum)` and `distance` of two Point3 includes the axial
component (wrong for hole alignment and for coaxiality). Fix: add builtins
`radial(p: Point3) -> Length` (distance from the z axis of p's datum, with
even-power-aware interval math), `coaxial(a: Datum, b: Datum, allowance, over:
Length)` (expands to the two-point test with points at `0` and `over` along
a's z, includes tilt; the zone is a radius, say so, and say the diameter form
is `2 * radial`), `parallel(a, b, allowance, over)`, and `angle_between(a, b)
-> Angle` (inexact, conservative). Add a `Datum` kind to the type grammar.
Specify that `x ^ 2` and `x * x` on the same affine form use the
square-aware interval. These are in scope because the owner's coaxial use
case cannot be written robustly without them.

### 5. gap: point arithmetic is contradictory or underspecified (geometry.md 3, language.md 3.8)

- language.md 3.8 shows `holes + Point2(5mm, 0mm)` ("`+ -` between an array
  of points and one point") but geometry.md 3 defines only Point + Vec and
  Point - Point = Vec. There is no `Vec2(...)` constructor anywhere, so a
  shift cannot be written. Fix: add `Vec2(x, y)`/`Vec3`; array + Vec shifts
  every element; array + Point is a G001-class error.
- Literals: `Point2(..) on top` is specified, `Point3(..)` is not. `p in D`
  is re-expression and `Point3(...) in D` reads as re-expressing a literal
  with no datum. Fix: state that `in D` applied to a literal is the typing
  form and applied to a typed point is re-expression; or give `Point3` the
  same `on D` spelling.
- `in` also starts a bound (`name in lo..hi`, language.md 3.4) and is not in
  the precedence list (language.md 5). Add `in` between `..` and `as`, and
  say a member beginning `IDENT in` is a bound only when the right side is a
  range.
- `p in D` on an array: unspecified. `Point2 on d` becomes Point3, but
  ScrewJoint `at`/`onto` and the language.md 3.8 pattern signatures are
  Point2 arrays. Fix: `in` maps over arrays; ScrewJoint takes `[Point2 on d;
  N]` with any `d`, and `aligned` converts both sides to `onto`'s datum and
  measures in its xy plane only (state the plane; today a z offset between
  datums silently adds to the distance).
- `rect_pattern(nx, ny, dx, dy)`: dx/dy are pitch or extent? "rows first"
  unclear. Fix: "dx, dy are pitches (centre to centre); points are ordered
  row by row, x varying fastest, from (-,-) to (+,+)". Also add an optional
  `rotation: Angle` argument to `rect_pattern` (circle_pattern has `start`);
  rotating a 2D pattern via a rotated datum is the right idiom but is not
  documented as such.

### 6. gap: `place ... for` index semantics and instance coverage (language.md 3, geometry.md 4)

`check c for w in wheel` binds `w` to an instance, but `place feet[i].base on
... corners[i] for i in feet` binds `i` as an index, and `x = corners[i].x`
splits a point into two Lengths. The same `for x in parts` syntax has two
meanings. Also no code covers "each part is placed at most once" (a double
placement or two `for` ranges overlapping), nor an array whose `for` misses
an instance. Fix: `place PART[i].d on EXPR for i in 0..count(PART)` (a Count
range form of `for`), and a form `D.at(point)` meaning offset to the point's
x, y (z) so the split disappears: `place feet[i].top on
base.underside.at(base.foot_sites[i]) for i in 0..count(feet)`. Add
double-placement (error, fixed) and unplaced-instance (warning: a placed
array not fully covered) to the G family; geometry.md 4 states a rule with no
diagnostic.

### 7. gap: placements and point patterns cannot be bound the way SOLIDWORKS dimensions them (geometry.md 6, adapters.md 3, bindings.md)

- Placement offsets have no canonical path (ir.md 2 stops at members;
  `Placement.ops[]` is untyped), yet geometry.md 6 binds them to
  `D1@Distance2@Mount.SLDASM`. A literal `40mm` inside `offset(...)` has no
  address. Fix: each placement op argument is a member with the path
  `Type/part[i].place.<op index>.<axis>` (`Drive/plate.place.0.x`); `for`
  placements get one per instance.
- Coordinates versus SW dimensions: a pattern of 2 holes at -25 and 25 is
  one SW dimension (50mm pitch) or a pitch plus a symmetry relation; a
  circle pattern is radius + count + angle, a linear pattern is spacing +
  count. geometry.md 3 makes each coordinate a parameter, which does not
  map 1:1 to one native dimension. Fix: a pattern produced by
  `rect_pattern`/`circle_pattern` is a generator with named members
  (`holes.pitch_x`, `holes.pitch_y`, `holes.radius`, `holes.start`); those
  bind; the derived coordinates are analysis-only (not emitted) unless the
  points are hand-listed. Say so in geometry.md 3 and 6.
- Sign: SW dimensions are magnitudes with direction set by the sketch; a
  negative coordinate has no native value. Fix: binding field `sign =
  "negate"` applied at emit/read; read back compares the transformed value.
- `flip` and the 90deg `rotate` are discrete mate alignments, not numbers;
  a `for` placement over a pattern is a component pattern (spacing and
  count), not four mates. Fix: state in geometry.md 6 which placement forms
  are bindable (offset terms: mate distance; non-90 rotate: angle mate) and
  that `flip`/exact rotations are verified-only (read reports a drift note,
  never a push), and that a `for` placement over a pattern binds to the
  native component pattern's generator members.

### 8. gap: no datum-to-native-frame binding, so R007 has nothing to compare (geometry.md 6, lints.md R007, ir.md 3)

Binding locators are `global | dimension | property` (ir.md 3, bindings.md
has no geometry kind). R007 compares a bound coordinate's sketch plane with
the datum's plane, but no locator says which SW reference plane or coordinate
system a datum is. Drift in the frame definition itself (somebody moves the
reference plane) is also unreadable. Fix: add locator kind `frame` (SW
coordinate-system or plane feature name); `datum top = origin.offset(z = h)`
binds `h` to the plane's offset dimension and the datum to `frame =
"Plane1@Rail.SLDPRT"`; R007 compares the sketch's plane reference to that
frame; add an info code for a datum read through by a bound coordinate with
no frame. Without it the adapter cannot "see" anything and R007 is
unimplementable in 0.1.0.

### 9. gap: interface datums have no contract, and swaps cannot repair patterns (geometry.md 2, language.md 3.2)

`datum mount` in an interface fixes a name only. Whether z points out of the
flange or into it, and where the shaft axis is, is convention; a swap to an
implementer with the opposite convention gives a flipped placement and
nonsense checks that still evaluate. And a pattern from an alternate can only
be compared, never reused: `plate.motor_holes = motor.mount_holes` is G001
(different datums) and there is no re-typing, no map, no closures. Fixes:
(a) an interface may state a datum relation as a bound, e.g. `datum shaft`
with `shaft.z == mount.z` (checked per implementer, I001 on violation);
(b) add explicit re-typing for points in a model that receives the part:
`motor.mount_holes as Point2 on motor_face`, valid only when the model has
both the part and its placement, otherwise G003.

### 10. gap: IR cannot carry variants or transforms for datums and placements (ir.md 3, geometry.md 4 and 6)

geometry.md 4 says "variants re-evaluate placements like any member", but
only `Member` has `variants`; `Datum`, `Placement` and `Point` have none, and
`Placement.ops` is untyped. `Datum.exact: bool` duplicates the exactness of
its op `Value`s and no composed transform is described. Fix: give `Datum` and
`Placement` the same `variants?` map keyed `"<slot>=<alt>"`; define one `Op =
offset{...} | rotate{...} | flip{...} | at{point}` shared by both; drop
`Datum.exact` or define it as the conjunction of the ops; state the composed
transform is derived (a 3x4 matrix of `Exact`, entries from sin/cos of exact
multiples of 90deg, else `Inexact` with a root), not stored. Add `Datum` and
`Placement` canonical-path forms to the ir.md 2 grammar.

### 11. polish: grammar and keyword consistency (language.md 2, 3, 3.1 vs geometry.md)

- `datum` and `place` are used as keywords in the grammar but are not in the
  reserved list in language.md 2 (nor `origin`, a predefined datum). Add
  them or say they are contextual, and make a member named `origin` an
  error.
- The grammar never defines `type` or `path`: `Point2 on top`, `Point3 in
  origin`, `[T; N]`, `Vec2`, `Datum`, and `feet[i].base` are accepted only by
  example.
- The 3.1 class table does not list `datum` or `place` for any class
  (`place` for `assembly`, `datum` for all three); an `ots` may declare
  datums and position arrays per the Motor above, so add them and note S001
  applies to datum offsets in an `ots`.
- `and`/`or` do not exist in the expression language (language.md 5), yet
  coaxial checks need conjunction; add `and or not` or state `for`-quantified
  checks per point as the idiom.
- G003 is not fixed while G001/G002 are; say why or make it consistent. The
  geometry.md 7 and lints.md G tables otherwise match.

### 12. polish: wording that mis-states or hides semantics (geometry.md 2 and 5)

- geometry.md 5: "half of the minimum clearance hole less the maximum major
  diameter" parses as `min_hole/2 - major`. Write the formula:
  `(min(clearance_hole) - max(major)) / 2`, and the floating form as the sum
  of both halves. State that `clearance_hole` is nominal so the user's hole
  class does not change the allowance, and that pairing is by index only (two
  orderings of one pattern give huge distances).
- Tolerances on rotation angles are allowed by quantities.md 5.1 but
  geometry.md 2 mentions only offsets; add one sentence: a toleranced angle
  makes affected coordinates conservative (quantities.md 5.4) and the 90deg
  exactness rule applies to the untoleranced nominal only.
- A tolerance after a pattern is one independent source variable per
  coordinate, so the pitch between two holes varies by twice the written
  tolerance; say it, and offer tolerance on the generator's pitch (finding 7).
- `.flip(x | y)` is `rotate(x = 180deg)`; keep as sugar but say so, and state
  that `place A on B` makes frames coincide (z directions equal), so a face
  mating against a face needs a `flip` in one datum.
