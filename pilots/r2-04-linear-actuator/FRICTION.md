# Pilot r2-04: linear actuator, friction (draft 1)

## 1. Verdict
Draft 1 resolves the blockers: pi-exact magnitudes and angle exponents let thrust, speed and resolution be written with no 2*pi and no inexact Counts, and families plus constructor args remove the invented models. Collapsed to 3 files. Remaining friction is at the edges of abstraction (class-level constants, dimensionless rounding), not in the quantity core.

## 2. Round-1 findings
- F1 RESOLVED: 360deg/1.8deg = 200, 600rpm*8mm/rev = 80mm/s, 32kHz all exact.
- F2 RESOLVED: all five 4.3 rows reproduce by hand; real code also needed `* 1rev` for work_out and resolution (Q008 otherwise).
- F3 RESOLVED: `.nominal` and postfix `[+-]` work.
- F4 RESOLVED: `g6 on bearing.bore [0um .. 20um]`; no rounding to h9/h7.
- F5 RESOLVED: `a / b` fit between two ots values.
- F6 PARTIAL: `ceil_to` fine; floor-to-Count still clumsy (below).
- F7 RESOLVED: A, Velocity, Inertia, Lead exist.
- F8 RESOLVED: `Drivetrain(motor = motor, screw = screw)` over interfaces.
- F9 PARTIAL: families and Material table work; length-independent members still need an instance (below).
- F10 PARTIAL: shared namespace and `use` cover the 608; cross-repo packages still OQ-11.
- F11 RESOLVED: chained comparisons, `as` precedence given.

## 3. New findings
1. awkward, language.md 3.1/4: `screw_length = ceil_to(.., screw.cut_increment)` feeds `part screw: TrScrew(length = screw_length)`; the instance is read in its own argument. Fix: allow `TrScrew::cut_increment` (class-level access to members not depending on params) or default-able params.
2. awkward, quantities.md 4.3: lever-arm exception makes `F*r` assignable to Torque, but is the stored exponent 0 or the kind's -1? If 0, `t / lead` later fires Q008. Fix: state that assignment re-tags to the target kind's canonical exponent.
3. awkward, quantities.md 5.5: floor to a Count needs `floor_to(x, 1) + 1`; bare step and bare `+ 1` on an anonymous dimensionless are unspecified (Q001?). Fix: `floor(x)`/`ceil(x)` returning Count, and anonymous dimensionless counts as Ratio in 2(b).
4. nit, quantities.md 4.3: `/ 1rad` to drop an exponent reads as a no-op. Fix: bless `x as Length` (strips angle exponent, kind-checked) alongside it.
5. nit, language.md 1/3: example `use std::fasteners::...` vs grammar `dotted.path::Name`; and enum variants `MetricThread::M3` vs D2's bare `Iso4762(M5, 25mm)`. Fix: pick one separator and say bare variants infer from the param type.
6. nit, language.md 3.1/3.5: std `Iso4762`/`Iso273` member names and `sum(carriage.mass)` over an array member are unspecified. Fix: list std table columns; define `sum(array.member)`.
