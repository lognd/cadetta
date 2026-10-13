# Friction: r3-02 bearing housing (draft 1.2)

## 1. Verdict
Draft 1.2 expresses the whole design literally, with 0 PROPOSED lines and
no blockers. The 7-argument `fn` became three `let` lines. Hand result is
unchanged: axial play [0.00, 0.64] mm, `axial_play_ok` undecided (C002).
Not used: ScrewJoint, Iso273, Materials (no screws or material in scope);
interface/alternates (see N2).

## 2. Prior findings
- F1-F3, F5-F7, F9, F10: RESOLVED (unchanged).
- F4 false C002 from correlated terms: RESOLVED (independence is the physics).
- F8 imports/class paths: RESOLVED (`use` grammar, dotted namespaces).
- N1 derived assembly values lose tolerance: RESOLVED (`let`, C004).
- N2 alias identity: RESOLVED (5.3: shares the variable).
- N3 matched-lot stacks: OPEN (OQ-9, deferred; independent analysis used).
- N4 `H7 on x` example: RESOLVED (3.7).
- N5 `Layout.x` on a never-instantiated model: PARTIAL, see N1 below.

## 3. New findings
1. awkward - language 4 / quantities 5.4: `Layout.x` on a plain model is
   never stated to be a singleton; whether `Layout.ring_support_dia` is
   one shared variable (so shoulder and spacer correlate) is implied only.
   Fix: say a non-parameterized model read by name is one implicit instance.
2. awkward - language 3.2 vs 3.4: a bound in an `interface` constrains an
   implementer's value, in an `ots` it is a vendor limit; so an interface
   cannot expose `shaft_abutment_dia >= 10mm`, and Housing cannot take an
   interface-typed bearing. Fix: let interfaces declare limit members.
3. awkward - stdlib 2: Din472 has no tolerance-class or edge-distance
   columns, so I wrote `H11`/`H13` and the 0.06 deviation by hand. Fix: add
   `groove_dia_class`, `groove_width_class`, `edge_dist` columns.
4. nit - stdlib 2: the `std.retaining::Din471 / Din472` row sits after the
   Material variants paragraph, outside the table. Fix: move it.
5. nit - language 3 grammar: no `check` production; `check n for x in p: e`
   (6) differs from r2's `check n: for ...`. Fix: add it to the grammar.
