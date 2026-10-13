# Language bindings, Python interop and SOLIDWORKS first

Status: current
Owner: cadetta
Decisions: none
Audience: contributors

- Date: 2026-10-13
- Ticket: none (pre-ledger)

## Context

The owner asked for an importable `cadetta` in Python for interop, a
repository that can take more languages later with JSON as the
catch-all, and named SOLIDWORKS as the CAD tool they use most. Draft
1.3 shipped Python only as a CLI wheel and planned SOLIDWORKS drive as
"later", in C#.

## Decision

1. Bindings are tiered (bindings.md 1). JSON is the catch-all: the CLI's
   `--format json`, the IR, the solver protocol and the adapter
   protocol reach every capability from any language. Native bindings
   are thin wrappers over the Rust crates, add no semantics, and prove
   it by replaying the conformance corpus.
2. Each binding lives in `bindings/<language>/` with its own native
   crate, package, tests and CI job. Bindings are workspace members but
   not default members, so the Rust gates need no other toolchain.
3. Python is the first binding: PyPI `cadetta` is a PyO3 extension plus
   a pydantic layer, and its console script runs the same Rust CLI as
   `cargo install cadetta`. The CLI is a library (`cadetta::run`) with a
   thin binary, so both entry points run identical code.
4. The Python package carries the solver decorator and the adapter base
   class. The solver helper dropped in the ticket re-plan comes back
   here.
5. SOLIDWORKS is the primary 0.1.0 target for emit, read and drive. Its
   adapter is Python on pywin32 COM, built on the adapter base. FreeCAD
   stays the CI reference for protocol conformance. (Superseded
   2026-10-13 by the draft 1.5 ADR, decision 13: the SOLIDWORKS adapter
   is C#, sharing a library with the add-in.)

## Consequences

- 0.1.0 grows a Python binding epic and a SOLIDWORKS adapter epic.
- SOLIDWORKS cannot run in CI. Its tests replay recorded COM
  transcripts, and a release needs a manual smoke run on a licensed
  machine.
- The Python package needs Python 3.11+ (abi3 wheels, `tomllib`).

## Alternatives considered

- A C# SOLIDWORKS adapter: the official interop assemblies are typed,
  but a second adapter SDK would have to be maintained. The calls
  cadetta needs (equation manager, parameters, custom properties,
  rebuild) are simple through pywin32. C# stays the fallback if COM
  marshalling through pywin32 proves unreliable.
- A binary-only PyPI wheel (the frob model): it gives no interop, which
  was the point of the request.
- A C ABI first: it serves more languages, but nobody needs it yet, and
  JSON covers them meanwhile.
