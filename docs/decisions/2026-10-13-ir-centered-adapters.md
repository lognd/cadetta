# A tool-agnostic IR sits between source and every CAD tool

Status: current
Owner: cadetta
Decisions: none
Audience: contributors

- Date: 2026-10-13
- Ticket: none (pre-ledger)

## Context

Cadetta must work with every common CAD tool and remove manual steps:
retyping values, importing parameter files, rebuilding, and transcribing
existing designs. CAD tools differ in what they expose: some only link a
text file (SOLIDWORKS equations file), some have full live APIs.

## Decision

All source compiles to a versioned IR (`cadetta.ir/1`). Lints read only
the IR. Each CAD tool is an adapter declaring any of three capabilities:
`emit` (write a parameter file), `read` (extract parameters from native
files into observed IR), `drive` (push values, regenerate, read back).
Adapters decide nothing: policy (tolerance point, units, precision) is
resolved before the IR reaches them.

## Consequences

- Lints behave identically whichever tool the data came from, and they
  run on imported designs before any `.cdt` source exists.
- Drift between source and CAD is a lint (R001), not a hope.
- Adding a CAD tool means writing one adapter, never touching lints.
- The IR schema is a public contract and needs a compatibility policy.

## Alternatives considered

- One emitter per tool straight from the AST: duplicates semantics in
  every backend, and nothing can read CAD files back.
- Pick SOLIDWORKS as the primary tool and treat others as exports: ties
  the data model to one vendor's flat-namespace parameters.
