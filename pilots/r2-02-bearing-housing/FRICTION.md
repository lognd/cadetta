# Friction: r2-02 bearing housing (draft 1)

## 1. Verdict
Draft 1 closes the round-1 blockers with no PROPOSED lines. Remaining
friction is derived analysis quantities and instance identity, not syntax.

## 2. Round-1 findings
- F1 one-sided fit on vendor bore: RESOLVED (`j5 on bearing.bore`, `H7 on bearing.outer_dia`).
- F2 `+0 -0.008` parses as arithmetic: RESOLVED (bracket).
- F3 vendor tolerance leaks / no nominal: RESOLVED (5.3 + `.nominal`).
- F4 false C002 from correlated terms: PARTIAL, see N1: the cancellation was an artifact of deriving Spacer.length; independence is now the physics.
- F5 role through references: RESOLVED (`#[role]`, 5.6; groove_dia keeps hole).
- F6 limit values as bounds: RESOLVED (`>=`, `in`, `satisfies`, `.min`).
- F7 `==` on toleranced: RESOLVED (`.nominal`).
- F8 imports/class paths: PARTIAL, see N5.
- F9 no quantifier: RESOLVED (`bearing[2]`, `for b in bearing`).
- F10 F003 numeric vs designation: RESOLVED (one-sided, nothing to compare).

## 3. Axial play, hand-evaluated (mm)
Sum1 = 40.1 [-0.05,+0.25] (groove_pos +-0.05, H13 [0,0.14], ring 1 [-0.06,0]).
Sum2 = 3 + 23 +-0.05 + 7 [-0.12,0] + 7 [-0.12,0] = 40 [-0.29,+0.05], independent.
Play = 0.1 [-0.10, +0.54] -> [0.00, 0.64]. Lower bound hits 0 exactly (holds,
zero margin); upper 0.64 > 0.5 so the check is undecided (C002), correctly, and
not conservative (all linear). Round-1's [0.05,0.47] assumed the spacer tracks bearing width; it does not.

## 4. New findings
1. awkward - quantities.md 5.3: a derived assembly parameter stores nominal only, so `axial_play: Length = ...` silently loses all tolerance and its checks pass trivially. I used a 7-argument `fn`. Fix: an `analysis` member kind (not stored, not emitted) or lint a check reading an arithmetic parameter over toleranced inputs.
2. awkward - 5.4/5.3: is an alias (`housing.groove_width = ring.groove_width`) the referent's variable or a new one? Changes whether the two are correlated. Fix: state aliases share the variable.
3. awkward - 5.4/OQ-9: two bearings are independent, but a stack that wants a matched lot has no syntax. Fix: defer, note in 3.5.
4. nit - 3.7: `H7 on x` (hole class one-sided) is only implied by case; examples show shaft only. Fix: add an example.
5. nit - 4: `Layout.x` on a never-instantiated model, and params typed by a concrete ots, are unstated. Fix: say so.
