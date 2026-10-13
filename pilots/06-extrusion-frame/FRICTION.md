# Friction report: 06-extrusion-frame (workbench.cdt)

## 1. Summary

Plain imperial lengths, `round_to`, `Count` sums and `Ratio` worked once I
learned the idioms. Fractions, feet-inches, angles and tolerance-by-reference
are where it hurts: the literal rules push `3/4in` into a silent
mis-parse, `deg` contradicts the exact-rational claim, and a toleranced
cut length cannot reference the vendor's cut tolerance. There is no way to
count `part` instances, so "legs -> bolts" is hand-maintained.

## 2. Findings

F1 (blocker) quantities.md 2, language.md 5. Wanted `3/4in`, `1/16"`.
Lexing gives `3 / 4in`, i.e. 3 divided by a length: dimension 1/L, no
error until it meets a Length. `3/4in * 2in` is `3/(4in)*2in` = 3,
dimensionless, accepted with no lint if the target is undeclared.
Forced `3in / 4` (reads wrong, and in `ots` "literals only" is arguably
violated). Suggest: fraction literal `3/4in` lexed as one token, or `3/4 * 1in`
declared legal; plus a lint (Q008) on `bare / quantity` in a Length position.
```
thickness = 3/4in      // one token, exact 3/4 in
```

F2 (awkward) quantities.md 2. `2'6"` lexes as `2'` then `6"`, a parse
error, not Q007. Forced `2ft + 6in`. Suggest compound literal
`2ft6in` or `2'6"` as a sum token.

F3 (blocker) quantities.md 1, 3. `deg` has no exact rational SI factor
(pi/180 rad), contradicting "every unit has an exact rational factor".
Is `45deg` exact? `cos(45deg)` is inexact regardless. Suggest: angles stored
as exact rational turns, or `pi` as a symbolic factor; state it.

F4 (awkward) language.md 3. No part arrays or instance counting; `leg_count:
Count = 4` restates four `part` lines and nothing checks they agree. D001 does
not see it. Suggest `part leg[4]: Leg` and `count(Leg)` (returns Count).
```
part leg[4]: Leg
total_bolts: Count = count(Gusset) * Gusset.bolts
```

F5 (blocker) language.md 5, quantities.md 5. `+-` is not in the expression
grammar. `round_to(x, s) +- 0.010in` is marked PROPOSED, and the deviation
cannot reference `Extrusion1010.cut_tol`, so the cut tolerance is restated
in three models and D001 will nag. Suggest `x +- Extrusion1010.cut_tol` as an
expression form with a tolerance-operator precedence.

F6 (awkward) language.md 5. `round_to` on a toleranced or inexact
argument is undefined (I rounded before attaching tolerance). Ties
(x.5 steps) have no stated rule; the inexact brace diagonal needs a
rounding decision made on an f64. Is the result exact? State: "exact
rational result; ties away from zero; error if the f64 is within 1 ulp of a tie".

F7 (awkward) language.md 3. No string/enum parameters. "1/4-20 UNC-2A", "6105-T5",
"clear anodize", "#7 drill" go in comments. Suggest `text` kind or `@note`.

F8 (nit) quantities.md 3. No `lb/ft^3`, `lbf/in` for imperial density;
used `g/cm^3`. `lb/ft` works as anonymous compound. No fractional-inch
display, so a cut list prints `45.9375in`, not `45-15/16`. Suggest `@display(in/32)`.

F9 (awkward) quantities.md 2. `1 - x` with `x: Ratio`: the bare `1` is not a
factor, so Q001; forced `100% - x`. Count/Ratio mixing: Count * Count
is anonymous dimensionless, assignable to `Ratio` (rule 4) without Q003.
Real error class: `total_bolts: Ratio = ...` passes. Also non-integer `Count`
(`4.5`) has no diagnostic.

F10 (nit) quantities.md 4. `sqrt(Area)` and `x ^ 2` kinds are not in the
kind algebra table; assumed Area and Length-dimension anonymous.
`atan2(y, x)` argument order and same-dimension requirement unspecified.

F11 (nit) language.md 4. `Extrusion1010`, `Plywood` : N001 canon for digits.
Also `check` in `assembly` cannot reference component-level params directly
without a part; I used `ShelfPanel.length`, which spec permits ("Other.param").

## 3. Hypothetical `cadetta check`

- Q006 (info) on Brace.raw_length, slope_angle, miter_angle, Gusset.hypotenuse,
  apex_angle, cut_length, total_extrusion_length, extrusion_mass: useful once, noise x8
  (it is transitive, not root-cause only).
- Q004 (warning) on miter and hypotenuse at any finite precision: useful.
- M002 (info) on `1in / 16`, `1in / 32`, `leg ^ 2 / 2`: noise (simple fractions are excluded, so should not fire).
- D001 (warning) x4 on `1in / 4`, `0.010in` repeated: useful for cut_tol, noise for 1/4 in diameters of different parts.
- D002: `Plywood.sheet_*` none. Perhaps `2 * rail_count` flagged: wrong (it is derived already).
- C001/C002: `forty_five` is `undecided`? atan2(3in,3in) vs 45deg inexact equality; spec silent. Wrong or C002 noise.
- S001: none; N002 on `Gusset.leg`? no.
- Not caught: count vs part-line mismatch; the `3/4in` mis-parse in an undeclared param;
  gusset bolt length longer than stack; bolt too short for T-nut engagement; mixing
  plywood nominal vs actual thickness (both are Length); brace long-point vs centerline;
  saw-cut tolerance vs fit.

## 4. Evaluation trace

(a) Brace.raw_length = sqrt(46^2 + 26^2) in = sqrt(2792) in = 52.839379254491625 in (f64,
inexact). round_to(.., 1/32in): 52.8394*32 = 1690.86 -> 1691 -> 1691/32 in = 52.84375 in.
Ambiguity: is result exact (it should be: a rational) and is Q006 still on it?

(b) Panel: WidthRail.length = 48in - 2in = 46in; minus 1/16 = 735/16 in = 45.9375in
(exact); round_to(...,1/32) no-op. Depth: 30 - 2 - 1/16 = 447/16 in. Area 328545/256 in^2;
two panels / (48in*96in) = 36505/65536 = 55.70 % utilization (exact Ratio); waste 100% -
that = 44.30 %. In SI: 1in = 127/5000 m, so 735/16 in = 93345/80000 m reduced.

(c) Hardware: gusset_count = 2*(4+4) = 16; bolts = 16*2 + 8*1 + 2*1 = 42; t_nuts 42. Total
extrusion length = 4*36 + 4*46 + 4*28 + 52.84375 = 492.84375 = 15771/32 in = 41.07 ft.

(d) apex_angle = atan2(3in,3in) = pi/4 rad as f64 0.7853981633974483, and `45deg` converts
to 0.7853981633974483 too, so `==` holds in doubles here by luck; `3/cos(45deg)` is
4.242640687119285 but `3*sqrt(2)` is 4.242640687119286. Two routes, two values. Ambiguous:
inexact equality semantics, `deg` exactness, and three-valued result for inexact operands.
