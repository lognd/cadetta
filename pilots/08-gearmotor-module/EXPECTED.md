# Pilot 08 expected results (hand-evaluated)

All std table values are ASSUMED plausible ISO figures, not read from a
real stdlib: M3 tap drill cut_75 2.5, cut_65 2.55; M4 3.3 / 3.4;
M5 4.2 / 4.3 (mm). Iso4762Head M5: head_dia 8.5, counterbore_dia 9.5,
counterbore_depth 5.4. Preferred lengths: 8, 10, 12, 16, 20. Engagement
factor: 6061 2.0 (IsoDefault) overridden to 1.5 by AcmeShop; 304 steel
1.0. Tap class: 6061 soft (cut_75), 304 hard (cut_65). Policy: bottom
clearance 0.5, lead-in 1.0, drill point 0.3 x drill dia. Screw masses
(1.1 g M3x8, 2.4 g M4x10, 5.3 g M5x16) and prices (0.08, 0.12, 0.15 USD)
are assumed too.

## 1. Screw joints

| Member | motor_screws, primary (M3, 6061) | motor_screws, Gm42B (M4, 6061) | rail_screws (M5, 304, cbore) |
|---|---|---|---|
| clamp | 3 | 4 | 10 |
| min_engagement | 1.5 x 3 = 4.5 | 1.5 x 4 = 6.0 | 1.0 x 5 = 5.0 |
| screw_length | at_least(7.5) = 8 | at_least(10) = 10 | at_least(15) = 16 |
| tap_drill | cut_75 = 2.5 | cut_75 = 3.3 | cut_65 = 4.3 |
| clearance_hole (medium) | 3.4 | 4.5 | 5.5 |
| thread_depth | (8-3)+0.5 = 5.5 | (10-4)+0.5 = 6.5 | (16-10)+0.5 = 6.5 |
| drill_depth | 5.5+1.0+0.75 = 7.25 | 6.5+1.0+0.99 = 8.49 | 6.5+1.0+1.29 = 8.79 |
| depth_available check | 7.25 <= 10 holds | 8.49 <= 10 holds | 8.79 <= 10 holds |
| counterbore | none | none | 9.5 dia x 5.4 deep |

Material difference: with the same thread, 6061 gets the 75% drill and
1.5 x d engagement (AcmeShop; IsoDefault would give 2.0 x d, so M3 needs
6.0 and screw_length 10), 304 gets the 65% drill (M5: 4.3 vs 4.2) and
1.0 x d.
Callout, primary: `M3 - 6H, 5.5 DP; drill 2.5 DP 7.25` (display rounding
assumed). Spec gap: with a counterbore, screw_length uses the full plate
thickness (16 mm) although 5.4 mm of it is the counterbore; the minimal
screw would be 10 mm (FRICTION 1).

## 2. Fits (emitted at mid)

| Fit | Primary Gm37A | Alternate Gm42B |
|---|---|---|
| hub_seat H7 on shaft | 6 H7 +0.012/0, shaft +0/-0.008: clearance 0..0.020; bore 6.006 | 8 H7 +0.015/0, shaft +0/-0.009: clearance 0..0.024; bore 8.0075 |
| required 0..0.03 | holds | holds |
| pilot H7 on pilot | 12 H7 +0.018/0, pilot -0.006/-0.017: clearance 0.006..0.035; bore 12.009 | 16 H7 +0.018/0, same pilot: clearance 0.006..0.035; bore 16.009 |
| required 0.005..0.05 | holds | holds |

## 3. Stack-up

wheel_gap = shaft_len - hub.width - plate.thickness - rail.thickness.
Four independent source variables, deviations 0.5 + 0.1 + 0.1 + 0.15 =
0.85 mm (linear, exact, not conservative).

- Primary: 32 - 8 - 10 - 10 = 4.0 mm, interval [3.15, 4.85]. Check
  `2mm <= wheel_gap <= 6mm`: holds.
- Gm42B: 34 - 8 - 10 - 10 = 6.0 mm, interval [5.15, 6.85]. Upper bound
  6 mm is exceeded for part of the interval: undecided (C002, exact).

## 4. Roll-ups, solver, requires

Mass (kg): motor 0.210 / 0.380; plate 2.70 g/cc x 80x60x10 = 0.1296;
hub 0.025; rail 0.320; motor screws 4 x 1.1 g / 4 x 2.4 g; rail screws
4 x 5.3 g = 0.0212. Primary 0.7102, alternate 0.9054.
Cost (USD): primary 24.50 + 38 + 9 + 14 + 0.32 + 0.60 = 86.42;
alternate 31.00 + 38 + 9 + 14 + 0.48 + 0.60 = 93.08 (+6.66, +7.7%).
Lead time (max): primary 3 wk (plate), alternate 5 wk (motor).

Solver `drive_torque` (script: analysis/drive_torque.py), robot_mass =
10 + 2 x module_mass:

| | robot_mass | torque_per_motor | 1.5 x | rated | verdict |
|---|---|---|---|---|---|
| primary | 11.420 kg | 0.684 N*m | 1.026 | 1.2 | torque_ok, slope_torque hold |
| Gm42B | 11.811 kg | 0.707 N*m (needs re-solve) | 1.061 | 2.0 | hold, with S003 (stale under variant) |

mass_budget (<= 0.95 kg): primary 0.7102 holds; Gm42B 0.9054 holds.
gap_ok: primary holds, Gm42B undecided. Interface bound
`rated_torque >= 0.8`: 1.2 and 2.0 hold.

## 5. Per-alternate verdicts

| | Gm37A (primary) | Gm42B |
|---|---|---|
| viable | yes | yes (I003: gap_ok undecided; S003 on solver) |
| lifecycle | eol 2027-03 | active |
| L001 | none (viable alternate exists) | n/a |

## 6. `cadetta impact --swap DriveModule/motor Gm42B` (abbreviated)

```
verdict: ok with warnings
  Gm42B is approved (alt list). Solver drive_torque is stale under this variant.

values that change
  direct   DriveModule/motor.shaft_dia      6mm [+0 -0.008mm] -> 8mm [+0 -0.009mm]
           DriveModule/motor.shaft_len      32mm [+-0.5mm]    -> 34mm [+-0.5mm]
           DriveModule/motor.pilot_dia      12mm -> 16mm
           DriveModule/motor.hole_pitch     31mm -> 35mm
           DriveModule/motor.mount_thread   M3 -> M4
           DriveModule/motor.rated_torque   1.2N*m -> 2.0N*m
  derived  DriveModule/hub.bore             6.006mm -> 8.0075mm
           DriveModule/plate.pilot_bore     12.009mm -> 16.009mm
           DriveModule/plate.mount_pitch    31mm -> 35mm
           DriveModule/motor_screws.tap_drill      2.5mm -> 3.3mm
           DriveModule/motor_screws.screw_length   8mm -> 10mm
           DriveModule/motor_screws.drill_depth    7.25mm -> 8.49mm
           DriveModule.wheel_gap            4.0mm [3.15..4.85] -> 6.0mm [5.15..6.85]  (let)
           DriveModule.module_mass          0.7102kg -> 0.9054kg
           DriveModule.robot_mass           11.4204kg -> 11.8108kg
           acme.drive::drive_torque.torque_per_motor  0.684N*m -> (stale; 0.707N*m if re-solved)

verdicts that flip
  DriveModule.gap_ok        holds -> undecided (C002)
  (no violations; rail_screws and rail fits unchanged)

suggested edit
  part motor: GearMotor = Gm42B alt [Gm37A]

CAD files to re-sync
  cad/motor_plate.SLDPRT   pilot_bore, mount_pitch, motor_screws.{tap_drill,drill_depth,thread_depth,callout}
  cad/wheel_hub.SLDPRT     bore
  cad/chassis_rail.SLDPRT  not stale

attestations invalidated
  tolerance_review (fit pilot, fit hub_seat, check gap_ok)  3 nodes
  source (Gm42B)  never given: O002 open

commercial deltas (USD)
  unit cost   86.42 -> 93.08   (+6.66, +7.7%)
  lead time   3wk -> 5wk
  moq         Gm42B: not declared
  mass        0.7102kg -> 0.9054kg  (no part lacks mass)
```

Promoting also leaves the primary Gm37A (eol 2027-03) in the alt list as
suggested; the author may drop it.
