# Positions and datums

Cadetta checks where features are relative to each other: hole patterns
that must line up across two parts, clearances between placed parts,
stack-ups that run through a placement. It is not a geometry kernel. It
has no solids, surfaces, shape interference, mate solving or GD&T
tolerance zones; CAD keeps the shapes and the mates, and cadetta drives
and checks their numbers.

A position is a set of Length quantities, so everything in
quantities.md applies unchanged: exact values, affine tolerances over
source variables, three-valued checks.

## 1. Example

```
use std::fasteners::ScrewJoint
use std::materials::Material

model Rail {
  height = 30mm
  material = Material::Al6061T6                // the host: tap drill and engagement
  datum top  = origin.offset(z = height)
  datum site = top.offset(x = 40mm)            // centre of the tapped pattern
  tapped: [Point2 on site; 2] = [Point2(-25mm, 0mm), Point2(25mm, 0mm)] [+-0.05mm]
}

model Bracket {
  thickness = 6mm
  datum base = origin
  holes: [Point2 on base; 2] = [Point2(-25mm, 0mm), Point2(25mm, 0mm)] [+-0.05mm]
}

assembly Mount {
  part rail: Rail
  part bracket: Bracket
  place rail.origin on origin
  place bracket.base on rail.site
  part screws: ScrewJoint(thread = M5, clamp = bracket.thickness, host = rail,
                          at = bracket.holes, onto = rail.tapped)
}
```

This is the pattern-datum idiom: the host declares a datum at the centre
of its hole pattern, declares the pattern on it, and the clamped part is
placed on that datum, so the nominal offset (40mm) is written once and
the joint never restates it. One joint covers the whole pattern (stdlib.md
3); it checks every pair `bracket.holes[i]`, `rail.tapped[i]`.

The joint holds. M5 at the medium class has a 5.5mm clearance hole and a
5.0mm major diameter, so the fixed allowance is (5.5mm - 5.0mm) / 2 =
0.25mm (section 5). Each coordinate is an independent source variable,
so per axis the difference of a bracket hole and its tapped hole is in
[-0.10mm, 0.10mm], and the distance is in [0, 0.1414mm]: 0.1414mm <=
0.25mm for every value in tolerance, all pairs.

Loosen the tolerances to `[+-0.2mm]` on the bracket and `[+-0.1mm]` on
the rail and the per-axis difference is [-0.3mm, 0.3mm]. The distance
reaches 0.424mm, over the allowance, yet 0 is also reachable, so the
joint is undecided: C002, not C001. Tighten either tolerance back and
it holds again; a larger thread alone does not decide it here (M6 gives
0.30mm, still under 0.424mm).
The result is conservative (quantities.md 5.4), and C002 says so.

## 2. Datums

A datum is a right-handed coordinate frame. Its xy plane is a sketch
plane and z is its normal.

- Every component has the datum `origin`: the CAD file's origin, so a
  position means the same thing in cadetta and in the part. `origin` is
  reserved; a member named `origin` is an error. `datum` is reserved
  (language.md 2) and every component class may declare datums
  (language.md 3.1); S001 applies to datum offsets in an `ots`.
- `datum NAME = EXPR` declares another, built from a datum of the same
  component: `origin.offset(x = .., y = .., z = ..)`, `.rotate(x | y | z =
  ANGLE)` and `.flip(x | y)`. Operations apply left to right, each in
  the frame built so far. `.flip(x)` is sugar for `.rotate(x = 180deg)`
  and reverses the normal.
- Offsets are Lengths and may carry tolerances, which become source
  variables like any parameter. Each offset and rotation argument is a
  Member with a canonical path (`Rail.site.offset.x`; ir.md 2), so it is
  emitted, bound and drift-checked like any other value.
- Rotations by whole multiples of 90deg are exact when the angle is
  untoleranced; any other angle makes the transform inexact
  (quantities.md 1, Q006). A toleranced angle is allowed (quantities.md
  5.1) and makes every coordinate carried through it conservative
  (quantities.md 5.4); the exactness rule applies to the untoleranced
  nominal only.
  Decided in the transform: only whole multiples of 90deg are exact, so
  60deg or 30deg are inexact although their sine is rational; an
  untoleranced inexact angle roots Q006 at sin or cos; a toleranced
  angle's entries are conservative intervals and carry a Q006 root only
  when the nominal is not a multiple of 90deg. Offsets stay exact affine
  forms (a tolerance is a source variable through exact rotations), and a
  coordinate carried through an inexact or toleranced rotation becomes an
  inexact interval.
- A datum is built only from `origin` or a datum declared earlier in the
  same component, so datum chains cannot cycle. The first operation of a
  kind in a chain has the path `Rail.site.offset.x`; the n-th (n > 0)
  carries an index, `Rail.site.offset[1].x`. `flip(x | y)` takes a bare
  axis and no value, so it has no Member; `flip(z)` is an error because
  it does not reverse the normal.
- An interface may declare datums (`datum mount`), and each implementer
  defines them. A placement on an interface-typed part therefore
  survives a swap: the alternate brings its own `mount`.
- An interface may relate its datums by a bound, written like any other
  bound: `datum shaft` with `shaft.z == mount.z` says the shaft axis is
  the flange normal. Each implementer is checked (C001 against that
  implementer on violation, like any interface bound),
  so a swap to an implementer with the opposite convention is an error,
  not a flipped placement that still evaluates.
  A datum bound compares axis directions exactly: it holds only when the
  implementer's frames, built from untoleranced whole multiples of 90deg,
  agree; a violation is C001 and any other rotation or a toleranced angle
  makes it undecided, C002. An implementer that omits a declared datum is
  I001, which no waiver suppresses.
- `Datum` is a type: a function or builtin parameter of type `Datum`
  takes a datum name (section 5). The type grammar is language.md 3.

## 3. Positions

- `Point2` and `Point3` are typed by a datum, the way quantities are
  typed by a kind: `Point2 on top` is a point in `top`'s plane,
  `Point3 on origin` a point in space. A literal takes its datum from
  the declared type, or is written `Point2(5mm, 0mm) on top` and
  `Point3(1mm, 2mm, 3mm) on top`; `on D` is the typing form of a
  literal, for both point types.
- `Vec2(x, y)` and `Vec3(x, y, z)` construct vectors. `Point - Point` is
  a vector in the same datum, `Point + Vec` a point, `Vec * s` scales by
  a dimensionless `s`. An array plus a vector shifts every element
  (`holes + Vec2(5mm, 0mm)`); an array plus a point is G001. Arithmetic
  across datums is G001 and never converts silently, just as named kinds
  never convert (Q003).
- `p in D` re-expresses a typed point `p` in datum `D`, through the
  component's own datums and, in an assembly, through placements
  (section 4). It never types a literal. `in` maps over arrays. A `Point2
  on d` becomes the `Point3` at z = 0 of `d`. Re-expressing a part's
  positions in a model that holds a different part (a swapped part's
  pattern, `motor.mount_holes as Point2 on motor_face`) is valid only
  where the model has both the part and its placement, otherwise G003.
- Members: `.x .y .z`; `distance(p, q)` after both are in one datum; it
  includes the axial component, so hole alignment uses `aligned` and
  axes use `radial` (section 5).
- Patterns return points on the datum of the declared type.
  `rect_pattern(nx, ny, dx, dy, rotation = 0deg)`: dx and dy are
  pitches (centre to centre), points are ordered row by row, x fastest,
  from (-,-) to (+,+), centred on the datum origin; rotating a pattern
  is `rotation` or a rotated datum. `circle_pattern(n, r, start)`
  (language.md 3.8).
- A tolerance written after a pattern or array applies to every
  coordinate of every point as one independent source variable per
  coordinate, so the pitch between two holes varies by twice the
  written tolerance. To tolerance the pitch itself, tolerance the
  generator's member (`rect_pattern(2, 2, 30mm [+-0.05mm], 40mm)`).
- Each hand-listed coordinate is a parameter with a canonical path
  (`Bracket.holes[1].x`). A pattern produced by a generator is a value
  with named members (`holes.pitch_x`, `holes.pitch_y`, `holes.radius`,
  `holes.start`, `holes.rotation`); those bind and emit, and the derived
  coordinates are analysis-only, never emitted, unless the points are
  hand-listed.

## 4. Placement

In an assembly, `place A on B` makes datum `A` of one part coincide with
datum `B`: the frames are equal, z directions included, so a face
mating against a face needs a `flip` in one datum. `B` is the assembly's
own datum or another part's, optionally built further: `place bracket.base
on rail.top.offset(x = 40mm).rotate(z = 90deg)`.

- Placements form a tree rooted at the assembly's `origin`: each part
  is placed at most once and no chain returns to itself. A cycle is G002.
  Because of the tree, placing parts is composition of transforms and
  never a solve.
- A second connection between two placed parts is a `check`, not a
  `place`. "Does the second bolted interface still line up, given every
  tolerance on the way round?" is the question the engineer wants
  answered, and a check answers it three-valued.
- Idiom: place a bolted part on a datum of its host that is the centre
  of the host's pattern (section 1), so the nominal position is never
  restated.
- An unplaced part's positions are usable only inside the part.
  Converting them out of it is G003.
- A subassembly placed in an assembly brings its placed parts along;
  instance paths compose (`Mount/sub/bracket.holes[1]`).
- Each instance of a part array is placed separately. The Count-range
  form places all of them: `place PART[i].d on EXPR for i in
  0..count(PART)`, where `i` is an index. `D.at(point)` is datum `D`
  offset to the point's x and y (and z for a `Point3`), so a point is
  never split into two Lengths:
  `place feet[i].top on base.underside.at(base.foot_sites[i]) for i in
  0..count(feet)`. `IDENT in` starts a bound only when its right side
  is a range (language.md 5), which is how `for i in 0..N` reads.
- A part placed twice (two places, or overlapping `for` ranges) is
  G006; a placed array that a `for` leaves with an instance unplaced is
  G007.
- Each placement op argument is a Member with the canonical path
  `Assembly/part.place.<op index>.<axis>` (`Drive/plate.place.0.x`;
  `Assembly/part[i].place...` for a for-placement); ir.md 2.
- Variants (language.md 3.2) re-evaluate placements like any member.

Evaluation rules (placement.rs):

- A part's frame is the anchor's frame, then the `on` operations, then
  the inverse of the placed datum (its operations reversed and
  negated). This is composition only; it never solves.
- A subassembly is a node whose datums are its own; its parts are
  placed relative to its `origin`, and instance paths join with `/`.
- Placing on a part that is itself never placed is G003.
- A `for` placement carries the `[*]` index in its canonical path
  (`Assembly/part[*].place.0.x`); an unindexed target covers every
  instance, so it overlaps any other placement of that part (G006).
- In the IR (op_members.rs) the `[*]` becomes the range variable
  (`Assembly/part[i].place.0.x`), since `[*]` is valid only in bindings;
  each argument is a `Place` Member with provenance `authored` and its
  expression text and span.

## 5. Checking positions

- Any check may read converted positions: `check fan_clear: (fan.face in
  origin).z - (guard.face in origin).z >= 3mm`. `and`, `or` and `not`
  combine conditions (language.md 5).
- `aligned(a, b, allowance)` holds when every pair `a[i]`, `b[i]` (same
  count, else G004; pairing is by index only, so two orderings of one
  pattern give large distances) is within `allowance` of each other.
  Both sides are converted to the datum of `b` (`onto` for a joint) and
  measured in its xy plane only; a z offset between the datums is
  ignored. Distance is nonlinear, so the result is conservative
  (quantities.md 5.4) and its C002 says so.
- `ScrewJoint` takes `at` (the clearance-hole positions in the clamped
  part) and `onto` (the threaded-hole positions in the host), each a
  `[Point2 on d; N]` with any `d`. It then checks `aligned` with the
  fixed-fastener allowance
  `(min(clearance_hole) - max(major)) / 2`,
  where `clearance_hole` is the nominal of the joint's clearance class
  (the user's hole tolerance does not change the allowance). A through
  bolt with a nut (`onto` a second clearance pattern, `nut = true`)
  uses the floating-fastener allowance: the sum of both halves,
  `(min(clearance_hole_at) - max(major)) / 2 + (min(clearance_hole_onto)
  - max(major)) / 2`. "Screw here" thus also proves that the holes line up.

### 5.1 Axes

Axis tests are builtins over the z axes of two datums:

| Builtin | Meaning |
|---|---|
| `radial(p: Point3) -> Length` | Distance of `p` from the z axis of its datum, `sqrt(x ^ 2 + y ^ 2)` with the square-aware interval below. |
| `coaxial(a: Datum, b: Datum, allowance, over: Length)` | The z axis of `a` lies within a radius `allowance` of the z axis of `b` along `over`: points at 0 and `over` on `a`'s z are carried into `b` and each must have `radial <= allowance`. Tilt is included. The zone is a radius; the diameter form is `2 * radial(..)`. |
| `parallel(a: Datum, b: Datum, allowance, over: Length)` | The lateral deviation of `a`'s z axis from `b`'s z direction, over a length `over`, is within `allowance`. |
| `angle_between(a: Datum, b: Datum) -> Angle` | Angle between the two z axes; inexact and conservative. |

`x ^ 2` and `x * x` of the same affine form use a square-aware interval
that is never negative, so `sqrt` of a sum of squares never fails on a
sign-straddling tolerance (quantities.md 5.4).

### 5.2 Bolt float

A bolted part in clearance holes is not located exactly: it can sit up to
the joint's float off nominal. Ignoring this would answer "holds" for
loops that can physically fail, so the float enters the placement.

- `ScrewJoint` exports `float: Length`, the allowance its `aligned`
  check uses, and takes `locating: Bool = false` (stdlib.md 3). It is
  `true` when a pilot, dowel or fit locates the part.
- The float applies to one placement edge: that of the clamped part
  (the part `at` belongs to) when its `place` target is built from the
  datum `onto` is declared on, so the joint is what holds the part to
  its placement parent. That edge then carries an implicit `[+-float]`
  on the x and y axes of `onto`'s datum: one source-variable pair per
  edge. If several non-locating joints hold the part to the same parent,
  the smallest float applies, because the part sits within the tightest
  pattern. If any joint on the edge is `locating`, there is no float.
  A joint between parts that are not placement parent and child adds no
  float; it is a loop closure, and its `aligned` check reads through the
  floats already on the tree.
- A joint's own `aligned` check reads its edge without that edge's float
  (the float is the slack that check proves); every other path through
  the edge reads it.
- A check tighter than the float that reads through such a placement is
  G005 (warning). Fix it with `locating = true` and a `fit` or dowel
  that really does locate the part, or loosen the check. Dowel pins are
  not in `std` 0.1.0; the escape hatch is the same.

## 6. In the IR and in CAD

- The IR holds each datum with its parent and ops, each point with its
  datum, and each placement as an edge of the part tree; datums and
  placements carry `variants` like members, share one `Op` form
  (`offset | rotate | flip | at`), and the composed transform is derived,
  not stored (ir.md, Datum and Placement). Digests cover all three
  (ir.md 4), so a moved datum makes everything that reads through it
  stale.
- CAD gets numbers, as everywhere else. Point coordinates bind to sketch
  dimensions measured from the datum (`D3@Sketch2@Bracket.SLDPRT`),
  datum offsets to reference geometry, and placement offsets to mate
  dimensions (`D1@Distance2@Mount.SLDASM`). The CAD tool keeps owning
  shapes and mates; cadetta drives their values and reads them back.
- Bindable forms: an offset term binds to a mate distance; a rotate by
  a non-90deg angle binds to an angle mate; `flip` and exact 90deg
  rotations are verified-only (read reports a drift note, never a push);
  a for-placement over a pattern binds to the native component
  pattern's generator members (`holes.pitch_x`, ...). A binding may set
  `sign = -1` for a coordinate whose native dimension is a magnitude.
- A datum binds to a native frame with the locator kind `frame` (a
  SOLIDWORKS coordinate-system or plane feature, e.g. `frame =
  "Plane1@Rail.SLDPRT"`); the datum's offsets bind to that plane's
  dimensions, and R007 compares a sketch's plane reference with the
  frame. Drift in the frame itself is then readable (adapters.md 3).
- Cadetta never reconstructs geometry from a CAD file. Where an adapter
  can see that a bound coordinate's sketch is not on the plane of its
  datum, or its dimension is not measured from the sketch origin, it
  reports R007.

## 7. Diagnostics

| Code | Name | Default | Meaning |
|---|---|---|---|
| G001 | datum-mismatch | error, fixed | Arithmetic or comparison between positions in different datums without `in`; also array + point. |
| G002 | placement-cycle | error, fixed | A placement chain returns to a part already placed; write the closing connection as a check. |
| G003 | unplaced-part | error, fixed | A position is converted out of a part that has no placement, or re-typed where the model lacks the part or its placement. |
| G004 | pattern-count-mismatch | error | `aligned` or a joint's `at`/`onto` pair patterns of different counts. |
| G005 | bolted-float-unlocated | warning | A check tighter than a joint's float reads through a placement carrying that float (section 5.2). |
| G006 | double-placement | error, fixed | A part or instance is placed more than once. |
| G007 | unplaced-instance | warning | A placed array leaves an instance without a placement. |
| R007 | binding-datum-mismatch | warning | A bound coordinate's native sketch is not on its datum's plane, or its dimension is not measured from the sketch origin (adapters that can see it). |

lints.md is the catalog; this table must agree with it.

## Open questions

- OQ-10 (closed 2026-10-13, geometry.md): positions are typed by
  datum, placements are a tree and loops are checks; frames and mate
  solving stay in CAD.
- OQ-17: True position (GD&T positional tolerance with material
  condition) as an alternative to coordinate tolerances? It would make
  `aligned` exact for round zones. Recommendation: after 0.1.0, as a
  tolerance form on points (`position 0.2mm at mmc`).
- OQ-18: Placements over a range of motion (`offset(x in 0mm..100mm)`)
  checked at their extremes, for slides and hinges? Recommendation:
  yes, after 0.1.0. The three-valued checks already handle ranges.
