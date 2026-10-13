# Friction, pilot 07 r3 (part swap, draft 1.2)

## 1. Verdict
Rewritten against draft 1.2 with 0 PROPOSED lines. The swap
output is deterministic for values, verdicts, edits and commercial
deltas; attestation counts and number formatting still need two rules.

## 2. Prior findings
1. Digest scope: PARTIAL (closure defined; instance rule unclear, new 1)
2. Per-alternate derived values: RESOLVED (variants, 3.2)
3. Viability, waivers, promotion, old primary: RESOLVED
4. Kinds, VolumeFlow, compound outs, lookups, sqrt(1 + x): RESOLVED
5. Output order and roll-ups: PARTIAL (max? and order done; schema, rounding open)
6. Lifecycle date, solver check NAME: PARTIAL (eol defined; active not)

## 3. New findings
1. blocker, ir.md 4 vs obligations.md 5.1 item 6: an instance's closure is "its member plus argument digests"; for an interface-typed slot it is unstated whether the primary (the `instantiates` edge) is in it, and the argument rule makes guard_ok (reads only guard_pitch) go stale on a fan swap. Fix: slot-instance closure includes the part node; arguments enter only via members that read them.
2. awkward, language.md 3.6 vs 3.2: a failed lookup is "an error at evaluation"; under variant evaluation it is unspecified whether the alternate is non-viable, undecided or the run fails. Fix: a variant error makes it non-viable, naming the error.
3. awkward, obligations.md 5.1 item 7 vs language.md 3.5: deltas are `unit_cost * count` sums and "lead time max" computed by cadetta, while the design also writes `sum?`/`max?` lets; two sources of truth, and whether `sum?` multiplies array instances is unstated. Fix: define deltas as the diff of the roll-up expressions, and state instance counting.
4. awkward, obligations.md 5.1: percentage rounding, float digits and `docs/schemas/impact.json` (absent) are unspecified. Fix: state decimals and ship the schema.
5. nit, language.md 3.2 lifecycle: the date's meaning is given only for `eol`; `active, "2026-10"` is undefined. Fix: say it is the verification date or forbid it.
6. nit, language.md 3.8: the function list shows scalar `min max` but not the `max(self, p)` / `max?` roll-up overloads; stdlib.md has no thread-forming screw family, so ScrewJoint, Iso273 and Materials did not fit here.
