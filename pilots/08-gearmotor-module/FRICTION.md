# Pilot 08 friction (gearmotor module)

## 1. Verdict

Draft 1.1 carries this design almost entirely: interface/alternate
variant evaluation, ScrewJoint across two materials, let stack-up and
roll-ups all worked on paper. The gaps are the std policy shape, hole
side one-sided fits, and solvers under variant evaluation. 5 PROPOSED
lines in design.cdt.

## 2. New findings

1. blocker | solvers.md 4, language.md 3.2 | a solver input that depends on a swapped part (robot_mass via mass?) has no value under variants and solvers never run in check | `variants per_alternate` and a `solve --variant` run; otherwise report such requires as undecided, not holds on a stale value.
2. awkward | stdlib.md 3 | counterbore joint: screw_length = at_least(clamp + engagement) ignores counterbore_depth, giving 16 mm instead of 10 mm | define clamp as the remaining grip, or subtract counterbore_depth.
3. awkward | language.md 3.7 | one-sided fit exists only for the shaft class (`p6 on x`); a hole class on a vendor shaft or pilot (hub bore, plate pilot bore) is the common case | allow `H7 on x` (class case picks the side).
4. awkward | stdlib.md 4 | FastenerPolicy members, units and drill_set shape are unspecified; AcmeShop body is guesswork | list members, or ship IsoDefault source as the template.
5. awkward | solvers.md 1 | solver `in` binds to `Type.member` or `Type/part.member` but assembly instance scope is unstated | state the path rules for inputs (instances need the assembly prefix).
6. nit | language.md 3.5 | no max/min roll-up (lead time); only sum, while impact --swap derives it implicitly | add `max?(self, p)`.
7. nit | stdlib.md 2 | Iso4762 `unit_cost?` has no source; BOM cost of screws comes from nowhere | policy supplies a price table, or screws show as missing in sum?.
8. nit | quantities.md 5.1 | negative-both tolerance `[-0.006mm -0.017mm]` (g6 pilots) is legal only by reading "upper first"; and `Material::Ss304` is not enumerated | one example, and list Material variants in stdlib.md.
