# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

User-visible changes land as fragments in `changelog.d/` (named
`<ticket-ulid>.<type>.md`, written with `frob ticket fragment`) and are
folded into this file when a release is cut.

## [Unreleased]

### Added

- Language and linter specification, draft 1.4, under `docs/spec`,
  including positions and datums, Rust-style packages and modules, and
  the Python binding.
- Cargo workspace scaffold: ten crates under `crates/` (units, syntax,
  fits, ir, solve, obligations, adapter, sema, lint, and the `cadetta`
  binary in `cadetta-cli`).
- The Python binding under `bindings/python` (`import cadetta` and the
  `cadetta` console script).
- `cadetta-freecad`, a Python external adapter for FreeCAD, under
  `adapters/freecad`.
- The golden corpus under `tests/golden`: eight realistic designs with
  hand-evaluated expected values, verdicts, diagnostics and artifacts.
