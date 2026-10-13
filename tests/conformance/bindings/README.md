# Bindings conformance fixtures

Fixtures for `docs/spec/bindings.md` (draft 1.4). Layout and the
`expect.json` format are in `../README.md`; this file lists what is
specific to this directory.

## Entry and fixture form

The Python binding is a PyO3 module, so almost every rule here is
Python-API behaviour that the Rust conformance runner cannot execute.
All pairs are `kind = "binding"`, `entry = "python"`, and are skipped
by the runner today (it runs only `kind = "source"`, `entry = "check"`,
runner ticket ~6BCVN48). They are executable specifications for the
Python binding test suite (ticket ~6ZMPC6J), which replays them.

A fixture is one API call:

| key | meaning |
|---|---|
| `api` | dotted callable under `cadetta` (`Project.open`, `Quantity.__add__`, `cadetta.quantity`) |
| `args` | positional arguments; quantity arguments are source literals parsed with `cadetta.quantity` |
| `files` | optional package tree (relative path to contents) staged as the project directory |
| other keys | call context (`role`, `edit`, `inputs`, `outputs`, `returns`, ...) |

Expectation blocks use `result` (`Ok`, `Err`, `raises`, `value`,
`protocol-error`) and descriptive keys (`QuantityError` = expected
`.code`, `error`, `value`, `nominal`, `package`, `modules`, ...).
`exit` is the CLI exit code the call maps to (0 clean, 1 error
diagnostics, 2 I/O, configuration or usage).

## Rules and diagnostics

bindings.md defines no diagnostic codes of its own. It reuses compiler
codes through `QuantityError.code` (Q002, Q003, E010) and the CLI exit
codes. Those codes are covered here at the binding surface; their
source-level fixtures live in `quantities` and `lints`.

## Not fixtured

- Tier 1 (JSON catch-all) and the C ABI are not binding behaviour;
  their schemas are covered by `config-cli` and `ir`.
- `to_pint` on a non-currency quantity needs the optional `pint` extra;
  the must-not-fire only fixes the `Result` shape.
- `member.variant` (bindings.md 3.1) has no pair: a hand-written
  variant package needs the alternate-part syntax of language.md 3.2,
  which the `language` fixtures own; file a follow-up there.
