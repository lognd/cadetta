# Cadetta is implemented in Rust; adapters may not be

Status: current
Owner: cadetta
Decisions: none
Audience: contributors

- Date: 2026-10-13
- Ticket: none (pre-ledger)

## Context

The core (parser, quantity kernel, IR, lints, built-in emitters, CLI) is
a compiler-shaped program that ships to CAD workstations, which are
mostly Windows machines without developer toolchains. CAD vendor APIs are
reachable from other languages: SOLIDWORKS through COM (C#), FreeCAD and
Fusion through Python.

## Decision

The core is a Rust workspace producing one static `cadetta` binary.
External adapters are separate executables in whatever language reaches
the vendor API, speaking JSON-RPC over stdio (adapters.md 5).

## Consequences

- One binary to install; no runtime on CAD workstations.
- Same toolchain as frob and grimble; the parser can follow their
  conventions.
- The adapter protocol and IR schema are a hard boundary and must be
  versioned (ir.md 5).

## Alternatives considered

- Python core: closer to FreeCAD and pywin32, but every workstation needs
  a managed interpreter, and exact rational arithmetic plus a parser are
  better served by Rust.
- C# core: natural for SOLIDWORKS, poor fit for every other target.
