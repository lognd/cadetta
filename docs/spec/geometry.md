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
use std.fasteners::ScrewJoint

model Rail {
  height = 30mm
  datum top = origin.offset(z = height)
  tapped: [Point2 on top; 2] = [Point2(-25mm, 0mm), Point2(25mm, 0mm)] [+-0.1mm]
}

model Bracket {
  thickness = 6mm
  datum base = origin
  holes: [Point2 on base; 2] = [Point2(-25mm, 0mm), Point2(25mm, 0mm)] [+-0.2mm]
}

assembly Mount {
  part rail: Rail
  part bracket: Bracket
  place rail.origin on origin
  place bracket.base on rail.top.offset(x = 40mm [+-0.3mm])
  part screws[2]: ScrewJoint(thread = M5, clamp = bracket.thickness, host = rail,
                             at = bracket.holes, onto = rail.tapped)
}
```

The joint proves both holes line up: each bracket hole, carried through
the placement into the rail's datum, must sit within half the radial
clearance of its tapped hole (section 5). Move the bracket's holes,
loosen a tolerance, or swap to M6, and the check is re-decided.

## 2. Datums

A datum is a right-handed coordinate frame. Its xy plane is a sketch
plane and z is its normal.

- Every component has the datum `origin`: the CAD file's origin, so a
  position means the same thing in cadetta and in the part.
- `datum NAME = EXPR` declares another, built from a datum of the same
  component: `origin.offset(x = .., y = .., z = ..)`, `.rotate(x | y | z =
  ANGLE)` and `.flip(x | y)` (a half turn about that axis, which reverses
  the normal). Operations apply left to right, each in the frame built
  so far.
- Offsets are Lengths and may carry tolerances, which become source
  variables like any parameter. Rotations by whole multiples of 90deg
  are exact; any other angle makes the transform inexact (quantities.md
  1, Q006).
- An interface may declare datums (`datum mount`), and each implementer
  defines them. A placement on an interface-typed part therefore
  survives a swap: the alternate brings its own `mount`.

## 3. Positions

- `Point2` and `Point3` are typed by a datum, the way quantities are
  typed by a kind: `Point2 on top` is a point in `top`'s plane,
  `Point3 in origin` a point in space. A literal takes its datum from
  the declared type, or is written `Point2(5mm, 0mm) on top`.
- `Point - Point` is a vector (`Vec2`, `Vec3`) in the same datum, and
  `Point + Vec` a point. Arithmetic across datums is G001 and never
  converts silently, just as named kinds never convert (Q003).
- `p in D` re-expresses `p` in datum `D`, through the component's own
  datums and, in an assembly, through placements (section 4). A
  `Point2 on d` becomes the `Point3` at z = 0 of `d`.
- Members: `.x .y .z`; `distance(p, q)` after both are in one datum.
- Patterns (`rect_pattern`, `circle_pattern`, language.md 3.8) return
  points on the datum of the declared type; a tolerance written after a
  pattern or array applies to every coordinate of every point.
- Each coordinate is a parameter with a canonical path
  (`Bracket.holes[1].x`), so it is emitted, bound and drift-checked
  like any other value.

## 4. Placement

In an assembly, `place A on B` makes datum `A` of one part coincide with
datum `B`. `B` is the assembly's own datum or another part's, optionally
built further: `place bracket.base on rail.top.offset(x = 40mm).rotate(z
= 90deg)`.

- Placements form a tree rooted at the assembly's `origin`: each part
  is placed at most once and no chain returns to itself. A cycle is G002.
  Because of the tree, placing parts is composition of transforms and
  never a solve.
- A second connection between two placed parts is a `check`, not a
  `place`. "Does the second bolted interface still line up, given every
  tolerance on the way round?" is the question the engineer wants
  answered, and a check answers it three-valued.
- An unplaced part's positions are usable only inside the part.
  Converting them out of it is G003.
- A subassembly placed in an assembly brings its placed parts along;
  instance paths compose (`Mount/sub/bracket.holes[1]`). Each instance
  of a part array is placed separately (`place feet[i].base on ...` for
  each `i`, or a `for` placement over a pattern: `place feet[i].base on
  origin.offset(x = corners[i].x, y = corners[i].y) for i in feet`).
- Variants (language.md 3.2) re-evaluate placements like any member.

## 5. Checking positions

- Any check may read converted positions: `check fan_clear: (fan.face in
  origin).z - (guard.face in origin).z >= 3mm`.
- `aligned(a, b, allowance)` holds when every pair `a[i]`, `b[i]` (same
  count, else G004) is within `allowance` of each other once both are in
  one datum. Distance is nonlinear, so the result is conservative
  (quantities.md 5.4) and its C002 says so.
- `ScrewJoint` takes `at` (the clearance-hole positions in the clamped
  part) and `onto` (the threaded-hole positions in the host). It then
  checks `aligned` with the fixed-fastener allowance: half of the minimum
  clearance hole less the maximum major diameter. A through bolt with a
  nut (`onto` a second clearance pattern, `nut = true`) uses the
  floating-fastener allowance, the sum of both halves. "Screw here" thus
  also proves that the holes line up.

## 6. In the IR and in CAD

- The IR holds each datum with its parent and exact transform, each
  point with its datum, and each placement as an edge of the part tree;
  digests cover all three (ir.md 4), so a moved datum makes everything
  that reads through it stale.
- CAD gets numbers, as everywhere else. Point coordinates bind to sketch
  dimensions measured from the datum (`D3@Sketch2@Bracket.SLDPRT`),
  datum offsets to reference geometry, and placement offsets to mate
  dimensions (`D1@Distance2@Mount.SLDASM`). The CAD tool keeps owning
  shapes and mates; cadetta drives their values and reads them back.
- Cadetta never reconstructs geometry from a CAD file. Where an adapter
  can see that a bound coordinate's sketch is not on the plane of its
  datum, it reports R007.

## 7. Diagnostics

| Code | Name | Default | Meaning |
|---|---|---|---|
| G001 | datum-mismatch | error, fixed | Arithmetic or comparison between positions in different datums without `in`. |
| G002 | placement-cycle | error, fixed | A placement chain returns to a part already placed; write the closing connection as a check. |
| G003 | unplaced-part | error | A position is converted out of a part that has no placement. |
| G004 | pattern-count-mismatch | error | `aligned` or a joint's `at`/`onto` pair patterns of different counts. |
| R007 | binding-datum-mismatch | warning | A bound coordinate's native sketch is not on its datum's plane (adapters that can see it). |

## Open questions

- OQ-17: True position (GD&T positional tolerance with material
  condition) as an alternative to coordinate tolerances? It would make
  `aligned` exact for round zones. Recommendation: after 0.1.0, as a
  tolerance form on points (`position 0.2mm at mmc`).
- OQ-18: Placements over a range of motion (`offset(x in 0mm..100mm)`)
  checked at their extremes, for slides and hinges? Recommendation:
  yes, after 0.1.0. The three-valued checks already handle ranges.
