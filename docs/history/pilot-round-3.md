# Pilot round 3: synthesis

All eight pilots were rewritten against draft 1.2, using `std` wherever
it fit, and asked for contradictions and ambiguities rather than
features. Each `r3-*` directory has the design and a FRICTION.md; the
spec changes that answer them are draft 1.3
(`docs/decisions/2026-10-13-draft-1-3-decisions.md`).

## 1. Scoreboard

| Pilot | PROPOSED lines | Blocker | Prior findings resolved / partial / open |
|---|---|---|---|
| r3-01 bracket | 1 (ScrewJoint `host`) | no | 9 / 1 / 0 |
| r3-02 bearing housing | 0 | no | all but N3 (matched lots, OQ-9) and N5 (partial) |
| r3-03 PCB enclosure | 5 (Point2/arrays, insert joint) | no | 6 / 2 / 2 |
| r3-04 linear actuator | 1 (vendor-tapped hole) | no | 5 / 1 / 1 |
| r3-05 SOLIDWORKS migration | 2 (bindings TOML, `ref_to`) | no | 6 / 2 / 0 |
| r3-06 extrusion frame (inch) | 1 (inch ScrewJoint) | no | 8 / 2 / 1 |
| r3-07 part swap | 0 | yes (instance closure) | 3 / 3 / 0 |
| r3-08 gearmotor module | 3 (policy enums, key-only table) | no | 6 / 2 / 0 |

Round 2 averaged 3 to 5 PROPOSED lines per pilot; round 3 averages 1.6,
and every PROPOSED line is either `std` data or a stated-nowhere
spelling, not a missing concept.

## 2. Findings and their draft 1.3 answers

| Finding | Pilots | Draft 1.3 |
|---|---|---|
| Instance closure for a swapped slot unclear; unrelated checks go stale | 07 (blocker) | ir.md 4: only arguments read, plus the part node for interface slots |
| D003 component- or member-level? | 01, 04 | language.md 4: member-level |
| `Type.member` on a parameter-free model: which instance? | 02, 03, 05, 06 | language.md 3.1: one implicit shared instance; N006 beside a `part` |
| Fits and checks under `process`: designed or drawn? | 03 | language.md 3.8: designed; `.drawn`; process tolerance is a source variable |
| `process`, `check`, key-only `table`, `solver` member missing from grammar | 02, 03, 08 | language.md 3 grammar |
| F004 vs C002 on partial overlap | 04 | F004 only when disjoint |
| Variant lookup miss | 07 | alternate non-viable, run continues |
| Swap deltas vs `sum?`/`max?`; instance counting; rounding | 07 | obligations.md 5.1: deltas are the roll-ups; formatting fixed |
| Assembly `let`; `let` as part argument; array sizes; `arr.member` | 01, 04, 06 | language.md 3.1, 3.3, 3.5 |
| Point2, arrays, patterns unspecified | 03 | language.md 3.8 signatures; frames still OQ-10 |
| Kind algebra table absent; `sqrt(Area)` | 04, 06 | quantities.md 4.1 table, Q011 |
| `atan2(3in, 3in) == 45deg` undecided | 06 | exact trig cases |
| Count to Ratio | 06 | assignable one way |
| ScrewJoint: tapped part unnamed; vendor threads; inserts; screw cost | 01, 03, 04, 08 | `host`, `tap = Vendor`, `screw_cost` |
| `MetricThread`, `TapClass`, `TapColumn` variants; `M2.5` | 03, 08 | listed; `M2_5` legal under N001 |
| std tables untoleranced; Din472 classes; `unit_cost?` contradiction | 02, 05, 08 | limits carry tolerances; `edge_margin`; member dropped |
| Inch fasteners "later" vs OQ-15 | 06 | `std` 0.2.0; OQ-15 resolved |
| Two globals, one `ref` dimension; TOML spelling | 05 | `ref_to`; spelling in adapters.md 3 |
| SW emitter: "differs from written value"; order through `let` | 05 | clarified |
| Interface cannot expose a vendor limit | 02 | expose it as a plain member and check against it |
| Root assembly with parameters | 01 | evaluated only with defaults; I004 |
| `active` lifecycle date | 07 | the date it was last verified |

## 3. Still open

- Matched-lot stacks (OQ-9, r3-02).
- Frames (OQ-10, r3-03).
- Allowance literals such as `+ 3.5mm` (r3-03); M002 covers factors only.
- The owner of file-level SOLIDWORKS globals (r3-05).
- Thread-forming screws in `std` (r3-07), listed as "later".
- `docs/schemas/impact.json`, which is written with the implementation.

## 4. Observations

- Round 3 found one blocker, against two in round 2. Like those, it is in
  the obligation graph, not in the language. The graph is now core, and
  its digest rules are the part most worth testing first.
- Four pilots independently asked what `Type.member` means on a
  parameter-free model. The rule was missing, not wrong, and all four
  guessed the same.
- `ScrewJoint` was used in five of eight pilots. Each of the four
  requests to extend it was a real shop case: a named host part, bought
  threads, inserts and cost. None of them needed new syntax.
