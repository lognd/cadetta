# Draft 1.4 decisions after the final review round

Status: current
Owner: cadetta
Decisions: none
Audience: contributors

- Date: 2026-10-13
- Ticket: none (pre-ledger)

## Context

The owner asked for one last pass so that every facet feels cohesive and
inherently useful. They added positions and frames to 0.1.0 "as long as
it's thought out", named SOLIDWORKS as their main tool, and asked for
Python interop. Five reviewers read the whole spec from five angles: the
SOLIDWORKS adoption journey, obligations used over time, a geometry
pilot, cross-document cohesion, and project scale with the Python
binding. They found 16 blockers, 38 gaps and 6 polish items
(`pilots/review/`).
The normative text is in `docs/spec/`.

## Decision

1. **Sync is a three-way merge.** Each locator is compared across base
   (the committed lock's `synced` record), design and a pre-push read.
   An edit made in the CAD tool is never overwritten silently; it is
   R001, and `import --update` writes it back into the source. Cadetta
   owns the dependency structure, so a push that replaces a native
   equation says so (R006).
2. **Live sessions are specified.** The adapter attaches to a running
   instance or starts a hidden one, refuses dirty or locked documents
   (R009), and saves only what it changed. If a rebuild fails it
   restores the previous values, and the lock records only locators
   that read back correctly (closes OQ-7 and OQ-8).
3. **Import is a reviewed proposal.** It writes a proposal, with
   `--accept` and `--update`, under a fixed naming rule. It binds the
   active configuration only, and imported positions stay scalars.
4. **Obligations have identity.** Each has an id, `KIND:GLOB`
   selectors, a definition or instance level, and a lock record
   grammar with a merge driver and `lock verify`. `why --stale` and
   `impact --since` answer "why is this stale" and "what did my change
   invalidate". Presentational attributes no longer cause staleness.
5. **One rule for `require`.** A `require` is never suppressed. An
   attestation signs off a verdict and never overrides it.
6. **One diagnostic record.** `cadetta.diag/1` is shared by lints,
   solvers and adapters. Text output is rustc-style by default; short,
   GitHub, SARIF and JSON formats exist; `check --watch` is the 0.1.0
   editor integration, and an LSP is deferred. New code families: E
   (resolution and evaluation), G005-G007, R008-R013, O004 and W005.
7. **A real project model.** One root holds many namespaces; adapters
   have ids and scopes; `[defaults]` sets project-wide policy;
   `--as-of` makes date rules reproducible. Parameterized components
   exist only as instances, and bindings name instance paths (`[*]` for
   all instances).
8. **Positions are first-class without a solver** (geometry.md, closes
   OQ-10):
   - Points are typed by datum, and parts are placed in a tree, so a
     loop is a check.
   - A screw joint is one per hole pattern, and its clearance float
     enters the placement unless the joint is locating.
   - Axis primitives cover coaxial and parallel; pattern generators
     carry bindable pitch members.
   - Datums bind to CAD planes and coordinate systems.
9. **Python is a full peer of the CLI.** Every verb has a method;
   `Result` is used only at I/O boundaries and operators raise; values
   are carried with their tolerances (closes OQ-16).
10. **Commercial data has a contract.** `std.commercial::Sourced`
    defines the names that roll-ups, swap deltas and `compare` read.

11. **An alternate's failures are I002.** A check that fails only under
    an approved alternate is reported once, as I002 on that alternate
    (a warning by default, configurable to an error); C001 and C002 are
    the primary's verdicts. A design that works no longer breaks its
    build because a backup part stopped fitting, but it is told so.
12. **A last verification pass caught 2 blockers.** Interface bounds
    could not express `==`, and the `Sourced` members could not be
    optional. Both are fixed, along with 11 smaller seams between
    documents (`pilots/review/`).

## Consequences

- adapters.md and config-cli.md roughly doubled. The SOLIDWORKS adapter
  epic needs stories for session handling, the three-way sync, import
  and naming.
- The IR gains a dimension vector and entities for datums, placements,
  interfaces, tables, waivers and processes. `Value.nominal` is always
  the designed value.
- Tickets must be re-planned against draft 1.4.

## Alternatives considered

- Two-way sync (design always wins): simpler, but it destroys work done
  in the CAD tool, which is where SOLIDWORKS users actually work.
- A constraint-solved assembly model: it duplicates SOLIDWORKS mates and
  needs a geometry kernel. A placement tree plus checks answers the
  tolerance question without either.
- Exceptions everywhere in Python: idiomatic for Python, but the house
  style reserves exceptions for programmer errors. A kind error in
  arithmetic is one; a missing file is not.
