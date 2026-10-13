# Friction r3: 06-extrusion-frame (draft 1.2)

## 1. Verdict
Draft 1.2 expresses the frame cleanly: 1 PROPOSED line (the inch
ScrewJoint). Fractions, `2'6"`, part arrays, tolerance-by-reference and
`Text` all work. Std inch fasteners being "later" bites exactly once:
no ScrewJoint, no Iso273-style helper; ots hand-modelling remains.

## 2. Prior findings
- F1 fraction literals: RESOLVED
- F2 feet-inches: RESOLVED
- F3 deg exactness: RESOLVED (pi^k); trig stays inexact
- F4 part arrays/count: RESOLVED
- F5 `+-` on expressions: RESOLVED
- F6 round_to rules: RESOLVED (ties, Q010)
- F7 text parameters: RESOLVED
- F8 imperial units/display: RESOLVED (lbf/in still absent)
- F9 `1 - x`, Count vs Ratio: PARTIAL (`1 - x` ok; Count to Ratio assignment unstated)
- F10 kind algebra for sqrt/^: OPEN (table still not in the spec)
- F11 N001 digits, Type.member in checks: PARTIAL (see new 3)

## 3. New findings
1. awkward, stdlib.md 1 vs OQ-15: `std.fasteners.inch` is "later" yet OQ-15 says it may be a package; US shops cannot use ScrewJoint at all. Fix: state the status and ship UNC 1/4-20 early.
2. awkward, quantities.md 4.1 rule 3: "the kind algebra table" is referenced but absent; kinds of `sqrt(Area)`, `x ^ 2`, `Area * Length` unspecified.
3. awkward, quantities.md 5.8: `atan2(3in,3in) == 45deg` is always undecided (inexact within 1e-9), though true; forced `abs(a - 45deg) < 0.001deg`. Fix: exact result for atan2 of equal exact args, or an `approx` form.
4. awkward, language.md 3.1 vs 4 and quantities.md 5.4: `Type.member` is defined only for parameterized components; for a model with several instances it is unclear whether it carries a source variable. Used `for r in arr` and `arr[0]`.
5. nit, stdlib.md 2: `Din471/Din472` row sits after the Material paragraph, outside the table; `Material` has no wood or 6105 alloy, so `Materials` is unused here.
6. nit, language.md 3.5: array size expressions (`[2 * (count(a) + count(b))]`) are not stated legal or checked for D003 against `count`.
