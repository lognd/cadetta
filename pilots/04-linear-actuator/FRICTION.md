# Pilot 04: linear actuator - friction report

Layout: 5 files, 3 namespaces. `parts/bearings.cdt` (acme.parts.bearings: the shared 608, importable by any project), `parts/motion.cdt` (acme.parts.motion: all other vendor parts), and `actuator/{drive,structure,assembly}.cdt`, all in `acme.actuator` (one design, one namespace; files split by concern).

## 1. Summary
Dimensions, fits, provenance and three-valued checks worked well for the mechanical layout. The kind system fails exactly where this design is hardest: rotation. Angle is dimensionless, `rev`/`deg` cannot be exact rationals, there is no `pi`, so every screw formula is either silently off by 2*pi or needs the `1rev` trick. Tolerance handling has no `nominal()`, and ISO-only fits cannot express bearing bores or vendor tolerances.

## 2. Findings
- F1 blocker, quantities.md 1/3/4.5: `deg`, `rev`, `rpm` need factors containing pi, but "every unit has an exact rational factor". Wanted: `360deg / 1.8deg == 200` exactly, `rpm * lead / 1rev` exactly 80 mm/s. Forced: angle values inexact, so thrust, speed, steps_per_rev go inexact (Q006 noise, possible 200.00000000000003 as a Count). Change: store magnitude as `r * pi^k`, rational r; pi cancels exactly. `pi` unneeded, `1rev` stays.
- F2 blocker (silent), quantities.md 4 rule 5: Angle dimensionless means `T*eta/lead` (forgot 2*pi) and `rpm*lead` (forgot /rev) compile clean and are wrong by 6.28x. Also `1rev * d` instead of `1rev * d/2` gives lead angle 10.3 deg not 19.99 deg, no diagnostic. Change: keep Angle in compound kinds: `lead: Length/Angle`; adding Q008 `angle-dropped` when a product loses an Angle factor without `/1rev` or `*1rev`.
- F3 awkward, language.md 3.1/4: no way to strip or set tolerance. `plate.width = Motor.face_width + ...` inherits +-0.2; `sqrt` of a toleranced value is undefined. Wanted `hole_pitch = nominal(Motor.hole_pitch) +- 0.05mm`. Forced: PROPOSED lines. Change: `nominal(x)`, `x +- tol` as an expression operator.
- F4 awkward, language.md 3.2: `fit` only takes ISO 286 classes. Bearing bore (0/-8 um, ISO 492) and vendor flange pilot (22 0/-0.05) are not ISO 286; I rounded to h9/h7 (2 and 1 um off from the datasheet) to satisfy F003, and the 608 bore had to become a `check`. Change: `fit x = H7/(0,-8um) on ...` or `fit x = H7/Bearing608.bore on ...`.
- F5 awkward, language.md 3.1: `ots` is literal-only, so ots-to-ots interfaces (coupler bore on motor shaft) cannot use `fit`; a `fit` also restates both sides and trips D002 against vendor facts. Change: allow `fit` between two ots params, no D002 on ots.
- F6 awkward, language.md 5: only nearest `round_to`; stock lengths need ceiling. `min + step/2` is a ceil that ties at the exact-fit case (357.5/5: which way?). Rounding mode undefined. Also no `floor` for hole counts, and bare `+ 1` is Q001 (only factor/exponent allowed). Change: `ceil_to(x, step)`, `floor(x)`, bare integers as Count operands in `+`.
- F7 awkward, quantities.md 3/4: no `Velocity`/`Acceleration` kinds, no `A`, `ohm`, `mH`, no inertia kind; no `unit`/`kind` declaration syntax though "packages may add units". `rated_current = 2A` is PROPOSED.
- F8 awkward, language.md 3.1/4: physics models (Drivetrain) must reference type names (`Nema17Motor.holding_torque`), assembly checks reference instances (`motor.holding_torque`). Two motors would be indistinguishable; Screw/EndBlock read `Actuator.travel`, inverting layering. Wanted constructor-style args: `part drive: Drivetrain(motor = motor, screw = screw)`.
- F9 awkward, language.md 3.1: ots cannot be sized. MGN12 rail at a length, T8 screw at a length are families; I invented `RailCut`/`Screw` models. Material, finish and "6061-T6" have nowhere to go but comments (no string param).
- F10 nit, ir.md/language.md: unspecified whether two files may share a namespace, how an `ots` in another namespace is named without `import ... as`, and no cross-project package mechanism; reusing the 608 in another repo means copying the file. Waivers are one code, one path: D001 coincidences (22 mm on motor boss, 608 OD, nut flange; 8 mm x3; 5 mm; 10 mm) need dozens of waivers. Add `waive D001 on Bearing608.*`.
- F11 nit: `as` is in quantities.md 4 but absent from the expression grammar; precedence unknown. Checks accept one comparison only, so ranges need two checks.

## 3. Hypothetical `cadetta check`
- Q006 info on lead_angle, friction_angle, efficiency, thrust, work_*, max_speed, step_rate, bolt_circle_diameter: useful once, then noise (all 9 stem from F1).
- Q004 on bolt_circle_diameter (43.8406...mm irrational): useful, but spec says "not representable at precision" is always true for irrationals; threshold undefined.
- D001: ~12 hits across ots values (22mm x3, 8mm x4, 5mm x2, 3mm, 6mm, 60mm): noise; common_values downgrade covers only some.
- D002 on T8LeadScrew.lead: wrong (vendor fact); waived.
- M003 on travel, rail_spacing, lengths used in checks: noise; capacity checks are not mating interfaces.
- C002 journal_clearance (g6 vs bore, overlap 3 um): useful, waived with expiry.
- F003 possibly on Bearing OD (0/-0.009 vs h5): numeric-equal, depends on whether F003 compares class or deviations (ambiguous).
- N004 none; S001 none; C001 none (all other checks hold).
- Uncaught by any lint: forgetting `starts` (lead=pitch=2 mm, 4x slower, passes everything); F2 cases; mu and torque_ratio assumptions; a rail length of stock that is not a catalog length; vertical orientation (lead angle 20 deg > friction 8.8 deg, so screw backdrives, no check type for that); critical screw speed.

## 4. Evaluation trace
1. Screw.min_length = 10+5+24+300+5+7+3 = 354mm = 177/500 m. +2.5mm = 356.5mm; /5mm = 71.3; round_to -> 71*5 = 355mm = 71/200 m exact. overhang = 355-354+3 = 4mm. Tie at min_length=355: 357.5/5 = 71.5 undefined.
2. running_torque = 9/20 N*m * 1/2 = 9/40 N*m. lambda = atan2(8mm, 2pi*3.5mm) = 19.9905 deg; phi = atan2(0.15, cos15) = 8.8270 deg; eta = tan(lambda)/tan(lambda+phi) = 0.66124. thrust = 2pi * 0.225 * 0.66124 / 0.008 = 116.85 N (inexact). moving_mass = 5 + 0.11275 + 0.18 + 0.025 = 5.31775 kg; required = 2*5.31775*(5+0.0980665) = 54.22 N. thrust_margin holds (2.15x); comparison of inexact vs exact: verdict rule undefined.
3. linear_resolution = 8mm/(200*16) = 1/400 mm exactly, only if 360deg/1.8deg cancels exactly (F1); Count 200 vs 200.0000000000000x. Ambiguous: is Count required to be an integer?
