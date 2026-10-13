# Draft 1.3 decisions after pilot round 3

Status: current
Owner: cadetta
Decisions: none
Audience: contributors

- Date: 2026-10-13
- Ticket: none (pre-ledger)

## Context

Round 3 (`pilots/SYNTHESIS-r3.md`) re-ran all eight pilots against
draft 1.2, using `std` wherever it fit and asking for contradictions
rather than wishes. Seven of eight reported no blocker; pilot r3-07
found one (instance closures for swapped slots). The rest were gaps in
stated rules, mostly ones the pilots had to guess the same way. The
normative text is in `docs/spec/`.

## Decision

1. Instance closures include only the part arguments a member reads,
   plus the part node itself for interface-typed slots. A swap makes
   exactly that slot's instance paths stale (ir.md 4).
2. Reference cycles (D003) are found between members, not components,
   so a model may take an argument computed from a joint that reads its
   own thickness.
3. A parameter-free component read by type name is one implicit
   instance shared project-wide; doing so beside a `part` of that type
   is N006.
4. Under a process, fits, checks and reads see the designed (as-made)
   value, `x.drawn` reads the drawn one, and the process's own
   tolerance is a source variable of the as-made value. Roles are rule
   names, per member only.
5. F004 is a fit wholly outside its required range; a partial overlap
   is C002.
6. An evaluation error under a variant makes that alternate non-viable;
   it never fails the run.
7. Swap commercial deltas are the language's own roll-ups, which count
   every instance; formatting is fixed.
8. Grammar gains `check`, `process`, `rule`, key-only `table` rows, and
   `solver` as a member; arrays, `Point2` and the patterns get
   signatures; the kind algebra table, Q011, I004 and exact trig cases
   are written down.
9. `ScrewJoint` gains `host`, `tap = Vendor` (threaded inserts and
   bought threads) and `screw_cost`; `std` publishes limits with their
   tolerances; thread and tap enums are listed; inch fasteners and
   inserts ship in `std` 0.2.0 (OQ-15 resolved).
10. Bindings files spell several locators as a `locators` array, and a
    `ref` dimension may name its global with `ref_to`.

## Consequences

- The sema crate needs a member-level dependency graph from the start;
  components are not the unit of cycle detection or staleness.
- `std` grows enum and tolerance data in 0.1.0; inch moves from "later"
  to 0.2.0, which US-shop users will notice first (pilot r3-06).
- Still open after round 3: matched-lot stacks (OQ-9), frames (OQ-10),
  packages (OQ-11), allowance literals such as `+ 3.5mm` (M002 covers
  factors only), and the owner of file-level SOLIDWORKS globals.

## Alternatives considered

- Component-level cycle detection: simpler, but it forced pilots r3-01
  and r3-04 to move hole sizing into the assembly.
- Fits reading the drawn value: it would make every fit depend on the
  printer calibration in the wrong direction (pilot r3-03).
- Failing the run on a variant lookup miss: one unusable alternate
  would hide the verdicts on every other one.
- Inch fasteners as a package: packages (OQ-11) are not in 0.1.0, so
  this would leave US shops with nothing for longer.
