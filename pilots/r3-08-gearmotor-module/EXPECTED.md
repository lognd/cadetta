# Pilot r3-08 expected results (hand-evaluated, draft 1.2)

Std table values are ASSUMED plausible ISO figures: M3 tap drill cut_75
2.5, cut_65 2.55; M4 3.3 / 3.4; M5 4.2 / 4.3 (mm). Pitches M3 0.5, M4
0.7, M5 0.8. Iso4762Head M5: counterbore_dia 9.5, counterbore_depth 5.4.
Policy AcmeShop: factor 6061 1.5, 304 1.0; 6061 soft (cut_75), 304 hard
(cut_65); bottom clearance 0.5; lead-in 2 pitches; drill point 0.3 x
drill dia; lengths 8, 10, 12, 16, 20. Screw masses (1.1 g M3x8, 2.4 g
M4x10, 3.3 g M5x10) are assumed. Screws have no price (stdlib.md 4).

## 1. Screw joints

| Member | motor_screws, Gm37A (M3, 6061) | motor_screws, Gm42B (M4, 6061) | rail_screws (M5, 304, cbore) |
|---|---|---|---|
| clamp | 3 | 4 | 10 |
| grip | 3 | 4 | 10 - 5.4 = 4.6 (CHANGED: was full 10) |
| min_engagement | 4.5 | 6.0 | 5.0 |
| screw_length | at_least(7.5) = 8 | at_least(10) = 10 | at_least(9.6) = 10 (was 16) |
| tap_drill | 2.5 | 3.3 | 4.3 |
| clearance_hole | 3.4 | 4.5 | 5.5 |
| thread_depth | (8-3)+0.5 = 5.5 | (10-4)+0.5 = 6.5 | (10-4.6)+0.5 = 5.9 |
| drill_depth | 5.5+1.0+0.75 = 7.25 | 6.5+1.4+0.99 = 8.89 | 5.9+1.6+1.29 = 8.79 |
| depth_available (10) | holds | holds | holds |
| counterbore | none | none | 9.5 dia x 5.4 deep |

Lead-in is 2 pitches (M3 1.0, M4 1.4, M5 1.6), assuming the Count is
multiplied by thread.pitch (FRICTION 4). Callout, primary:
`M3 - 6H, 5.5 DP; drill 2.5 DP 7.25`.

## 2. Fits (emitted at mid): unchanged from the prior version

| Fit | Gm37A | Gm42B |
|---|---|---|
| hub_seat H7 on shaft, required 0..0.03 | bore 6.006, clearance 0..0.020, holds | bore 8.0075, clearance 0..0.024, holds |
| pilot H7 on pilot, required 0.005..0.05 | bore 12.009, clearance 0.006..0.035, holds | bore 16.009, clearance 0.006..0.035, holds |

## 3. Stack-up: unchanged

Primary 4.0 mm [3.15, 4.85], gap_ok holds. Gm42B 6.0 mm [5.15, 6.85],
gap_ok undecided (C002, I003).

## 4. Roll-ups, solver, requires

Mass (kg): plate 0.1296, hub 0.025, rail 0.320. Primary: 0.210 + motor
screws 0.0044 + rail screws 0.0132 = 0.7022. Gm42B: 0.380 + 0.0096 +
0.0132 = 0.8774.
Cost (USD), screws missing from the sum: primary 24.50 + 38 + 9 + 14 =
85.50; Gm42B 92.00 (+6.50, +7.6%). Lead time `max?`: 3 wk (plate) vs
5 wk (Gm42B). Missing-parameter note lists motor_screws and rail_screws.

Solver drive_torque, now declared inside DriveModule (no PROPOSED):

| | robot_mass | torque_per_motor | 1.5 x | rated | verdict |
|---|---|---|---|---|---|
| Gm37A | 11.4044 kg | 0.683 N*m | 1.025 | 1.2 | torque_ok, slope_torque hold |
| Gm42B, after `solve --variants` | 11.7548 kg | 0.704 N*m | 1.056 | 2.0 | hold |
| Gm42B, before it | 11.7548 kg | no result | - | 2.0 | undecided, S006 |

mass_budget: 0.7022 and 0.8774 both hold (<= 0.95). Interface bound
rated_torque >= 0.8: holds.

## 5. Per-alternate verdicts

Gm37A: primary, eol 2027-03. Gm42B: viable. I003 for gap_ok (and for
torque_ok, slope_torque until solved, with S006).

## 6. `cadetta impact --swap DriveModule/motor Gm42B` (abbreviated)

```
verdict: ok with warnings
  S006: run `cadetta solve --variants drive_torque` for Gm42B.

values that change (derived)
  motor_screws.{tap_drill 2.5->3.3, screw_length 8->10, drill_depth 7.25->8.89}
  hub.bore 6.006 -> 8.0075, plate.pilot_bore 12.009 -> 16.009
  wheel_gap 4.0 [3.15..4.85] -> 6.0 [5.15..6.85]
  module_mass 0.7022kg -> 0.8774kg, robot_mass 11.4044 -> 11.7548kg
  drive_torque.torque_per_motor 0.683 -> unsolved (0.704 once solved)

verdicts that flip
  gap_ok holds -> undecided (C002); torque_ok, slope_torque holds -> undecided (S006)

commercial deltas (USD)
  unit cost 85.50 -> 92.00 (+6.50, +7.6%), screws not priced
  lead time 3wk -> 5wk
```
