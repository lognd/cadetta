# Friction: Pi 4B enclosure (pilot 03), round 3 vs draft 1.2

## 1. Verdict
Draft 1.2 expresses the design well: `process` removes the manual
compensation, `fit a / b [range]` covers the heat-set insert, `let` holds
the height stack-up, `count()` ties the standoffs to the pattern. 5
PROPOSED lines (all Point2/array or ScrewJoint-with-insert). No blockers.

## 2. Prior findings
- F1 point/array: PARTIAL (types named; no literal, pattern signature, accessors)
- F2 frames: OPEN (OQ-10; `Case.datum + p` is workable but unchecked)
- F3 process compensation: RESOLVED (3.8)
- F4 derived members in ots / wrong class: RESOLVED
- F5 round_to on toleranced: RESOLVED (5.5); tolerance of drawn value: PARTIAL
- F6 percent / anonymous factors: RESOLVED (`5%` = 1/20)
- F7 source kinds: RESOLVED (`@measured/@assumed`)
- F8 interference fit: RESOLVED (3.7)
- F9 count tied to geometry: RESOLVED (`count(array)`)
- F10 magic allowances (`+ 3.5mm`): OPEN (M002 covers factors only)

## 3. New findings
1. awkward, 3.8 + 3.7 + 6: fits and checks on a process-compensated parameter
   read the designed or the drawn value? Insert fit differs by 0.2mm.
   Fix: say fits/checks use designed (as-made); add `.drawn`.
2. awkward, 3.8: `process` has no grammar production; rule role names are
   open (`slot`, `lip_in`?) but 3.11 lists only `hole/boss/...`; a
   `#![slot]` default hits positions too. Fix: add grammar; roles = rule names.
3. awkward, 3.8 + 5.3: `hole_comp [+-0.05mm]`: does the drawn value carry the
   tolerance, and is it a source variable of Case.insert_hole? Fix: state it.
4. awkward, stdlib 3 + 2: `ScrewJoint.into` takes a Material only (no
   insert; `std.inserts` later); `MetricThread` variants unlisted, `M2.5` is
   not a legal name; no attribute site for roles on joint members.
5. awkward, 3.3/3.8: `Point2` has no constructor, `.x`, arithmetic with
   arrays or `rect_pattern` signature; no map, so shifting 4 holes needs broadcast.
6. nit, 3.1/3.5: `Case.datum` reads a non-parameterized model as a singleton
   while `part base: Base` also instances one; are `Base.x` and `base.x` the
   same? Fix: say models read by type are singletons or require a part.
