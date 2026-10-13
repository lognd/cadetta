# 08 gearmotor module

A drive module: a gearmotor (primary Gm37A, approved alternate Gm42B) bolted
to a machined 6061 plate, the plate bolted to a 304 rail, and a wheel hub
bored to the shaft. Two `ScrewJoint`s (one per hole pattern, with `at`/`onto`,
`host` and a counterbore), a shop `FastenerPolicy`, two `fit`s, a wheel-gap
stack-up, commercial roll-ups and a Python solver for wheel torque.

Exercises: stdlib.md 3, 4, 6; geometry.md 2, 4, 5; language.md 3.2;
solvers.md 1, 4; bindings.md 3.3.

Notable: under Gm42B `gap_ok` is undecided (I003) and no solver result is
recorded for the new robot_mass, so `torque_ok` and `slope_torque` are
undecided with S006. `analysis/drive_torque.out.json` uses the placeholder
digest "blake3:TBD" (not computable by hand); it is the primary's result.
Iso4762 screw masses are assumed; screws have no price, so `module_cost` omits them.
