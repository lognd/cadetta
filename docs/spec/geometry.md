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
  place rail.origin at origin
  place bracket.base at rail.site
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
  component: `origin.offset(x = .., y = .., z = ..)`, `.offset(POINT)`
  (section 4), `.rotate(x | y | z = ANGLE)` and `.flip(x | y)`.
  Operations apply left to right, each in the frame built so far.
  `.flip(x)` is sugar for `.rotate(x = 180deg)` and reverses the normal.
- A datum is a frame, not a face. Convention for mounting datums: z
  points along the stack, out of the host and into the part mounted on
  it. A part modelled up from its base gets that for free (`datum base
  = origin`), and a host's mounting face points outward (`datum top =
  origin.offset(z = height)`), so `place bracket.base at rail.top`
  needs no flip. A datum that faces the other way (a motor hung under
  a plate) says so once, with `.flip(x)` in its declaration.
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
  never convert (Q003). A vector written `Vec2(x, y)` is free and takes
  the datum of the point or vector it meets; `Point - Point` is fixed to
  the points' datum, and a fixed vector meeting another datum is G001.
- `p in D` re-expresses a typed point `p` in datum `D`, through the
  component's own datums and, in an assembly, through placements
  (section 4). It never types a literal. `in` maps over arrays. A `Point2
  on d` becomes the `Point3` at z = 0 of `d`. Re-expressing a part's
  positions in a model that holds a different part (a swapped part's
  pattern, `motor.mount_holes in motor_face`) is valid only where the
  model has both the part and its placement, otherwise G003.
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
  `holes.start`, `holes.rotation`, and the counts `holes.count_x`,
  `holes.count_y`, `holes.count`); those bind and emit, and the derived
  coordinates are analysis-only, never emitted, unless the points are
  hand-listed. A count member emits to a native pattern's instance
  count, so the count is written once.
- Re-expression evaluation (reexpress.rs): `p in D` carries `p` out of its
  datum into the root frame and back into `D`, composing the placements
  on the path. Inside a part the root is the part's `origin`; in an
  assembly it is the assembly's, and a placed part's datum is named
  `part.datum` (`sub/part.datum` through a subassembly). A `Point2`
  always yields a `Point3`, lifted to z = 0 of its datum before the
  carry. A datum naming a part the model lacks or never placed, or a
  part-local datum met in an assembly, is G003; an unknown datum of a
  placed part is an error without a code. Off-90deg rotations make
  coordinates inexact (marked, Q006).
- Evaluation decisions: a rotation of a pattern follows the datum rotation
  rules (exact for whole multiples of 90deg, inexact and marked
  otherwise); a toleranced generator member shares one source variable
  across the points it moves; a tolerance after a pattern adds one fresh
  variable per coordinate; an omitted `rotation` has no member; a point
  count of zero is an error. The counts (`nx`, `ny`, `n`) are structural:
  they must be exact whole numbers known before evaluation (a literal,
  or a parameter-free constant), and they fix the array length. They are
  also Count members (`count_x`, `count_y` for `rect_pattern`, `count`
  for `circle_pattern`) that bind, push and drift like any member; a
  count changed in CAD is R001, and `import --update` rewrites its
  literal.

### 3.1 Position tolerance

Drawings usually tolerance a hole pattern by true position: each hole
must lie within a circle of a given diameter centred on its true
position. Cadetta writes that as a tolerance bracket on a `Point2` value
or array:

```
holes: [Point2 on base; 4] = rect_pattern(2, 2, 40mm, 30mm) [position 0.2mm]
pin: Point2 on base = Point2(10mm, 5mm) [position 0.1mm]
```

- The length after `position` is the zone diameter, as the drawing's
  feature control frame states it, regardless of feature size. Material
  condition modifiers (the MMC bonus) are OQ-17.
- Each point gets one independent disc source variable: its true
  position plus a displacement of at most half the diameter, in any
  direction in its datum's xy plane. The points of a pattern vary
  independently; the generator's members are the drawing's basic
  dimensions. Tolerancing a generator member as well is E006 (a pattern
  is toleranced one way), as is combining `position` with a coordinate
  tolerance on one value.
- `position` applies to `Point2` values and arrays only. On a `Point3`
  it is E006 in 0.1.0: a cylindrical zone needs an axis direction, which
  a point does not carry.
- Evaluation (quantities.md 5.4): the disc `(u, v)`, `u^2 + v^2 <= 1`,
  adds `(d/2) u` to x and `(d/2) v` to y. An exact rotation maps a disc
  to a disc of the same size, so re-expression through placements keeps
  it exact. `.x` or `.y` alone ranges over `[-d/2, d/2]` about nominal,
  exactly; a scalar that mixes both coordinates of one disc (after an
  off-90deg rotation) is bounded by the Euclidean norm of the pair,
  inexact unless one coefficient is zero.
- `distance`, `radial` and `aligned` treat disc terms exactly when the
  discs lie in the plane being measured: the farthest a sum of discs
  reaches is the sum of their radii. A disc tilted out of that plane is
  bounded by its radius, which is sound and marked conservative. With
  `[position 0.1mm]` on both patterns of section 1, each pair's worst
  offset is 0.05mm + 0.05mm = 0.1mm, against 0.1414mm for the
  coordinate box `[+-0.05mm]`. The result is conservative only when the
  chain has an inexact rotation or other nonlinear terms.
- Decided in implementation (`cadetta-geometry` `zone`): `distance` and
  `aligned` bound a difference of point coordinates by its per-disc Gram
  matrix. The result is exact (rational square-root bounds, outward at
  2^-32) when every disc is isotropic, no box term is present and the
  discs lie in the measured plane: the extreme squared distance is
  `|c|^2` plus `(max(0, m - R))^2` to `(m + R)^2`, with `R` the sum of
  radii. Tilted, elliptic or box-mixed discs, and forms with a pi factor,
  fall back to a conservative enclosure: `|c|` plus or minus the sum of
  spectral radii and the box norm. An axial offset outside the disc plane
  adds to `|c|^2` unchanged. Under an inexact rotation `Transform::apply`
  bounds each disc's contribution by its Euclidean reach
  `sqrt(U^2 + V^2)` instead of `|U| + |V|`; the disc identity is then
  dropped, so later `aligned` and `distance` over that value are
  conservative. Exact quarter turns keep the identity.
- A position zone is not a dimension. It is not emitted or bound; the
  nominal coordinates and the generator members are.

## 4. Placement

In an assembly, `place A at B` makes datum `A` of one part coincide with
frame `B`: the origins are equal and so are all three axes. `B` is the
assembly's own datum or another part's, optionally built further:
`place bracket.base at rail.top.offset(x = 40mm).rotate(z = 90deg)`.

- `at` never flips anything, because a datum is a frame, not a face
  (section 2). With the stacking convention a placement reads the way
  the parts sit; a part that hangs the other way says so once in its
  own datum, and an interface states its datums' directions as datum
  bounds, so every implementer is checked.
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
  restated. More generally, put every offset in a datum of the host and
  place op-free: the CAD side is then a coincident coordinate-system
  mate with no number in it (section 6), and nothing about the
  placement is bound by hand.
- An unplaced part's positions are usable only inside the part.
  Converting them out of it is G003.
- A subassembly placed in an assembly brings its placed parts along;
  instance paths compose (`Mount/sub/bracket.holes[1]`).
- **Point offsets.** `D.offset(p)`, with `p` a point typed on `D`, is
  `D.offset(x = p.x, y = p.y)` (and `z` for a `Point3`), so a point is
  never split into two Lengths in the source. It must be the first
  operation after the named datum, because `p` is measured in `D`; a
  point on another datum is G001. Its arguments are Members whose
  expression is the point.
- **Arrays place elementwise.** `place feet.top at
  base.underside.offset(base.foot_sites)` places instance i of `feet`
  at point i of `base.foot_sites`. A `place` whose target names an
  unindexed part array is evaluated once per instance, and every
  array-valued operand of the anchor (a point array, a datum of an
  unindexed part array) is indexed with the same i. Each must have the
  part array's count, else G004. An anchor with no array operand would
  put every instance in one frame and is G004 as well. An indexed
  target (`feet[2].top`) places that one instance, and its anchor must
  be single-valued (G004).
- A part or instance placed twice (two placements, or an array
  placement and an indexed one of the same instance) is G006. A part
  array whose instances are placed one by one and not all of them is
  G007.
- Each placement op argument is a Member with the canonical path
  `Assembly/part.place.<op>.<axis>`, numbered like datum ops (section
  2): the first `offset` is `Drive/plate.place.offset.x`, the second
  `place.offset[1].x`, a rotation `place.rotate.z`. An array placement
  lowers to one placement per instance, with integer indexes
  (`Assembly/feet[2].place.offset.x`, ir.md 2); its Members share the
  source expression and span, and bind per instance or with `[*]`.
- Variants (language.md 3.2) re-evaluate placements like any member.

Evaluation rules (placement.rs):

- A part's frame is the anchor's frame, then the anchor's operations,
  then the inverse of the placed datum (its operations reversed and
  negated). This is composition only; it never solves.
- A subassembly is a node whose datums are its own; its parts are
  placed relative to its `origin`, and instance paths join with `/`.
- Placing on a part that is itself never placed is G003.
- Instance i of an array placement is evaluated with every array
  operand indexed by i; instance paths read `Assembly/feet[2]`. In the
  IR (op_members.rs) each argument is a `Place` Member with provenance
  `authored` and its expression text and span.
- G007 is judged per part array that has at least one indexed
  placement and no array placement: the instances in 0 to count - 1
  that no placement covers are listed in the diagnostic.

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

`coaxial` and `parallel` are check conditions (they give a verdict, and
C002 when undecided), carried through the same frame table as `in`, so
placement tolerances and flips flow in; a flipped z axis is still
coaxial and parallel (the axis is a line, not a ray). `angle_between`
is the angle of the carried z direction, `atan2(lateral, axial)`, in
`[0, pi]`: exact when no tolerance is involved and the direction is
along or across the axis (0, pi/2 or pi), otherwise inexact, marked
`atan2` (Q006), with outward-padded bounds from the tolerance box.

### 5.2 Bolt float

A bolted part in clearance holes is not located exactly: it can sit up to
the joint's float off nominal, and turn within the clearances. Ignoring
this would answer "holds" for loops that can physically fail, so the
float enters the placement.

- `ScrewJoint` exports `float: Length`, the allowance its `aligned`
  check uses, and takes `locating: Bool = false` (stdlib.md 3). It is
  `true` when a pilot, dowel or fit locates the part.
- A non-locating joint whose two parts (the part `at` belongs to and
  the part `onto` belongs to) are placement parent and child, in either
  order, puts its float on that edge, whatever datums the placement
  names: every datum of a part moves with the part. The edge then
  carries, in the frame of `onto`'s datum:
  - a translation `[+-float]` on x and on y (one source-variable pair
    per edge);
  - a rotation about the z axis through the centroid of `onto`'s
    nominal points, by `[+-theta]` with `theta = 2 * asin(float / s)`,
    where `s` is the largest distance between two `onto` points, and
    `theta = 180deg` when `float >= s` (one source variable per edge).

  Both bounds are sound: each hole can move at most `float`, so the
  pattern's centroid moves at most `float` and its widest pair turns at
  most `theta`. `asin` is inexact, so coordinates carried through the
  rotation are conservative intervals (section 2).
- `s` is measured on `onto` (for a through bolt with `nut = true`
  too). A pattern that does not fix rotation has `theta` = 180deg, any
  check whose value depends on that rotation is undecided, and the
  joint is G008 (warning). That is the case for a one-point pattern,
  for one whose points all coincide (`s = 0`), and for one whose
  `float` is at or above its spread (`float >= s`), where the bolts may
  turn a half turn. Fix it with a second fastener, a locating feature and
  `locating = true`, or accept it with a waiver.
- If several non-locating joints hold one edge, each alone bounds the
  motion, so the edge takes one joint's whole bound (translation,
  `theta`, centroid and frame together): the joint with the smallest
  `theta`, then the smallest `float`, then the first in canonical-path
  byte order. Bounds from different joints are never mixed, because
  rotations about different centres are different motions. If any
  joint on the edge is `locating`, the edge has no float.
- A joint between parts that are not placement parent and child adds
  no float; it is a loop closure, and its `aligned` check reads through
  the floats already on the tree.
- A joint's own `aligned` check reads its edge without that edge's float
  (the float is the slack that check proves); every other path through
  the edge reads it.
- A check tighter than the float that reads through such a placement is
  G005 (warning). Fix it with `locating = true` and a `fit` or dowel
  that really does locate the part, or loosen the check. Dowel pins are
  not in `std` 0.1.0; the escape hatch is the same.
- Implementation note. Sema lowers each joint while it builds the frame
  tables (`boltfloat.rs`): the joint's `at` and `onto` datums resolve to
  their owning parts, and when those are placement parent and child the
  joint is recorded on the child's edge with the pattern points read
  from the nominal placement. Each joint owns three source variables
  (x, y, theta), memoised per canonical path, so every table built for
  the same joint shares them. Joints inside a sub-assembly component
  are lowered onto that sub-assembly's own placement edges, read through
  the root frame table with the datum ids prefixed by the node path.

## 6. In the IR and in CAD

- The IR holds each datum with its parent and ops, each point with its
  datum, and each placement as an edge of the part tree; datums and
  placements carry `variants` like members, share one `Op` form
  (`offset | rotate | flip`; the point form of `offset` lowers to
  `offset` with Members whose expression is the point), and the
  composed transform is derived, not stored (ir.md, Datum and
  Placement). Digests cover all three (ir.md 4), so a moved datum makes
  everything that reads through it stale.
- Implementation notes. Re-expression runs twice, once for the
  tolerance form and once for the exact nominals, so inexact results are
  marked (Q006). A point-form placement reads x, y and z from one point
  expression. A read through a nested part chain
  (`sub.bracket.holes[1]`) composes each part's placement. Re-expression
  of an indexed part instance (`leg[1].top in origin`) uses a frame
  table with that one instance placed, so each side of `in` may name one
  instance of a slot; two different instances of one slot in a single
  `in` (`leg[2].top in leg[1].top`) is an error, as is an index outside
  the array. A member read through an indexed slot (`rail[0].length`) is
  the part type's own member.
- CAD gets numbers, and positions are checked by transform. Cadetta
  never reconstructs geometry; reading a frame's or a component's
  transform is reading numbers the tool exposes. The mapping
  (adapters.md 3.4, solidworks.md 4):

  | Source | Native counterpart | Driven by | Verified by |
  |---|---|---|---|
  | datum | a frame (SOLIDWORKS coordinate system) | its composed transform, or bound dimensions of the feature defining it | comparing the frame's transform with the composed one (R001) |
  | hand-listed point | a sketch point | sketch dimensions measured from the datum | the bound dimensions (R001); R007 |
  | pattern generator member (pitch, radius, count) | the native pattern's dimensions and instance count | the bound locators | the bound locators (R001) |
  | placement | the component instance | bound mate distance or angle dimensions, when the placement has ops | comparing the component's transform with the composed placement (R001) |
  | array placement over a pattern | a pattern-driven component pattern | the part's pattern (nothing to bind) | transform per instance (R001) |

- `flip` and exact 90deg rotations are never pushed; the transform
  comparison verifies them. A binding may set `sign = -1` for a
  coordinate whose native dimension is a magnitude.
- The idiom that removes duplication: datums carry every number, CAD
  frames are created and driven from them (`[[adapter]] datums =
  "create"`), and parts are mated coordinate system to coordinate
  system with no number in the mate. Then a moved datum moves the CAD
  frame on the next `sync`, the mate follows, and the transform check
  confirms it.
- Where an adapter can see that a bound coordinate's sketch is not on
  the plane of its datum, or its dimension is not measured from the
  sketch origin, it reports R007.

## 7. Diagnostics

| Code | Name | Default | Meaning |
|---|---|---|---|
| G001 | datum-mismatch | error, fixed | Arithmetic or comparison between positions in different datums without `in`; also array + point, and a point offset whose point is not on the datum it offsets. |
| G002 | placement-cycle | error, fixed | A placement chain returns to a part already placed; write the closing connection as a check. |
| G003 | unplaced-part | error, fixed | A position is converted out of a part that has no placement, or re-typed where the model lacks the part or its placement. |
| G004 | pattern-count-mismatch | error | `aligned` or a joint's `at`/`onto` pair patterns of different counts; an array placement's operands do not have the part array's count, or an indexed placement's anchor is array-valued. |
| G005 | bolted-float-unlocated | warning | A check tighter than a joint's float reads through a placement carrying that float (section 5.2). |
| G006 | double-placement | error, fixed | A part or instance is placed more than once. |
| G007 | unplaced-instance | warning | A part array placed instance by instance leaves an instance without a placement. |
| G008 | rotation-unconstrained | warning | A non-locating joint whose pattern does not fix rotation (one point, coincident points, or float at or above the spread) carries float onto a placement edge, so the part may turn freely about the screw (section 5.2). |
| R007 | binding-datum-mismatch | warning | A bound coordinate's native sketch is not on its datum's plane, or its dimension is not measured from the sketch origin (adapters that can see it). |

lints.md is the catalog; this table must agree with it.

Implementation notes. G001 is decided statically from declared types:
each operand of `+ -`, a comparison or `distance` is classified by its
`Point2 on D` type (or by the shapes of the `let`s it was built from,
with `p in D` retagging to `D`), so values need no run-time datum. A
comparison or `distance` that raises G001 suppresses the evaluator's own
failure for the same expression. G003 fires on `x as Point2 on D` when
`x` reads through a part slot that no `place` positions. G007 is a
warning at the part slot. The placement float of section 5.2 is
evaluated by `cadetta-geometry` (`float.rs`): an assembly carries one
`EdgeFloat` per placement edge with the joints between its two parts,
the edge takes the whole bound of the selected joint (translation,
`theta`, centroid together; `theta` rounded up to `1e-9 pi`), and a
joint's own `aligned` check places the tree with `FloatMode::Without`.
The caller lowers the `onto` points into the frame of the placement
datum and supplies the source variables. G005 is emitted by sema when
an undecided check comparison reads a form that depends on a joint's
float source variables (so `locating = true` or a looser check keeps it
quiet). G008 (`float::unconstrained_rotation`) is emitted by sema once per
non-locating joint when a check re-expresses through the floated edge.

## 8. Conformance decisions

Rules the fixtures under `tests/conformance/geometry/` pin where the
sections above were silent (decided 2026-10-13, pending owner review):

- `flip(z)` is an error (E006): `flip` takes `x` or `y` only, because
  flipping the normal is `flip(x)` or `flip(y)` and the third axis
  would be a no-op on the frame.
- A datum whose definition names a datum declared later, or one that
  does not exist, is E002 (not a reference); datums are built from
  earlier datums only.
- A pattern generator with a zero point count is E006.
- A datum name that the placed part does not declare (`bracket.nope`)
  is E002, the same as any unknown member.
- `circle_pattern` numbers its points counter-clockwise starting at
  `+x` of its datum: point 0 is at `(r, 0)`.
- `aligned`, `coaxial`, `parallel` and `angle_between` are boolean
  or angle functions usable directly as a check condition, in addition
  to comparisons and `x satisfies P` (language.md 6).

## Open questions

- OQ-10 (closed 2026-10-13, geometry.md): positions are typed by
  datum, placements are a tree and loops are checks; frames and mate
  solving stay in CAD.
- OQ-17 (narrowed 2026-10-13, draft 1.5 ADR): true position regardless
  of feature size is in 0.1.0 (section 3.1). Open: material condition
  modifiers, where the zone grows as the hole departs from MMC
  (`[position 0.2mm at mmc]`), which couples a point's zone to its
  hole's size. Recommendation: after 0.1.0.
- OQ-18: Placements over a range of motion (`offset(x in 0mm..100mm)`)
  checked at their extremes, for slides and hinges? Recommendation:
  yes, after 0.1.0. The three-valued checks already handle ranges.
