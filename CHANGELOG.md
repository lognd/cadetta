# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

User-visible changes land as fragments in `changelog.d/` (named
`<ticket-ulid>.<type>.md`, written with `frob ticket fragment`) and are
folded into this file when a release is cut.

## [Unreleased]

### Added

- Language and linter specification, draft 1.3, under `docs/spec`.
- Cargo workspace scaffold: ten `cadetta-*` crates under `crates/`
  (units, syntax, fits, ir, solve, obligations, adapter, sema, lint,
  cli) and the `cadetta` binary in `cadetta-cli`.
- `cadetta-freecad`, a Python external adapter for FreeCAD, under
  `adapters/freecad`.
- Pilot projects under `pilots/` used to exercise the specification.
