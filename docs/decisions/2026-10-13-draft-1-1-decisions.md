# Draft 1.1 decisions after pilot round 2

Status: current
Owner: cadetta
Decisions: none
Audience: contributors

- Date: 2026-10-13
- Ticket: none (pre-ledger)

## Context

Round 2 (`docs/history/pilot-round-2.md`) re-ran pilots 02, 04 and 05 against
draft 1 and added pilot 07 (part swap). Round 1's blockers are closed;
pilot 07 found two new blockers in the swap queries. The owner asked
whether screws and other basic components should be computed for the
user. The normative text is in `docs/spec/`.

## Decision

1. A member `let` is an analysis value (stack-ups): it keeps its full
   affine tolerance and is never emitted or bound. Arithmetic parameters
   are features and stay their own source variables. C004 flags the
   confusion.
2. A pure reference is the referent's source variable, not a copy.
3. Node digests come in two forms: own and closure (Merkle over deps).
   Obligations and waivers bind closure digests.
4. Alternates are checked by variant evaluation: one part slot at a
   time, everything downstream re-evaluated, and differing values kept
   per variant in the IR. Viable means nothing violates. Waivers may be
   scoped `for alt`. A violated require blocks the swap.
5. Fasteners and other shop knowledge go in `std`, written in ordinary
   `.cdt`, and not in syntax. `ScrewJoint` derives holes, tap drills,
   depths, screw length and callout from thread, clamp and material.
   Shop policy is an overridable `FastenerPolicy` (`#![default(...)]`).
6. Namespaces are dotted and `::` enters one (`use std.fasteners::X`).
   Bare enum variants are allowed where the type is known. Check names
   are required.
7. Smaller additions:
   - assignment and `as` set the canonical angle exponent;
   - `floor`/`ceil`/`round` return Count;
   - VolumeFlow;
   - kind expressions;
   - table `at_least`/`at_most`/`nearest` with `.key`;
   - `Type.member` for members that do not depend on parameters;
   - defaults for component parameters;
   - `fmt`.
8. Adapters:
   - several locators per path;
   - only bound paths are emitted, and as values, never expressions;
   - read never raises Q001;
   - one format document per emitter.

9. Draft 1.2 (pilot 08):
   - solver results are keyed by input set; a variant without one is
     undecided (S006), never given the primary's result;
   - `ScrewJoint` sizes from `grip`, net of the counterbore;
   - `H7 on x` is a one-sided hole fit;
   - `FastenerPolicy` is a listed interface with `Table<K>` members;
   - `max`/`min` roll-ups.

## Consequences

- `std` is a deliverable in 0.1.0 (metric only), and doubles as the
  largest test of the language: anything `std` needs, users get.
- The IR gains `analysis`, `variants`, `closure` and multi-locator
  bindings; the obligations crate computes Merkle closures.
- Tickets must be re-planned against draft 1.1.

## Alternatives considered

- A `joint`/`screw` keyword: it bakes one shop's rules into the grammar
  and cannot be cited or overridden.
- Cross-product variant evaluation: exact for interacting alternates,
  but exponential; slot-at-a-time covers the swap question users ask.
- Following derivations for every parameter in checks: it would treat a
  machined spacer as tracking the bearing it was sized from, which is
  physically wrong (pilot r2-02).
