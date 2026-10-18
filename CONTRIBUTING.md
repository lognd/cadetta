# Contributing to cadetta

cadetta is a parameter language and linter for 3D CAD. It is a Rust
workspace (`crates/cadetta-*`, binary `cadetta`), bindings (`bindings/python`,
`bindings/c`) and external CAD adapters (`adapters/freecad`, `adapters/fusion`
in Python, `adapters/solidworks` in C#). The work is tracked with
[frob](https://github.com/lognd/frob) tickets and the architecture with
grimble. Thanks for considering a contribution.

## Spec first

The specification under `docs/spec` is the product right now. Any change
to behavior updates `docs/spec` in the same change, and the golden corpus
(`tests/golden`) and conformance corpus (`tests/conformance`) must
keep agreeing with it. If you
find an ambiguity, contradiction or gap, open a "Spec question /
ambiguity" issue before writing code. Do not edit `design/` or `tickets/`
by hand; tickets are written only through `frob ticket ...`.

## Quick start

- Fork, clone, and add `lognd/cadetta` as `upstream`.
- Rust: install the 1.98 toolchain (`rustup toolchain install 1.98.0`
  with `rustfmt` and `clippy`). Build with `cargo build --workspace`.
- Python adapter: install [uv](https://docs.astral.sh/uv/), then
  `cd adapters/freecad && uv sync`.
- With frob installed (`uv tool install frob`), `frob check` is the
  full local gate and `frob test` runs the tests your diff reaches.
  Neither is required to contribute, but CI runs the same tools.

## Gates

Run these before opening a PR.

Rust, from the repository root:

```bash
cargo fmt --all --check
cargo clippy --workspace --exclude cadetta-py --exclude cadetta-capi --all-targets -- -D warnings
cargo nextest run --workspace --exclude cadetta-py --exclude cadetta-capi
cargo build -p cadetta-capi -p cadetta && cargo test -p cadetta-capi
```

Python binding, inside `bindings/python` (`uv run maturin develop` first):
`uv run ruff check`, `uv run python scripts/gen_models.py --check`,
`uv run pytest`. Editor grammars: `uv run editors/generate.py --check`.

Python, inside `adapters/freecad`:

```bash
uv run pytest
uv run ruff check
uv run ruff format --check
```

Fusion adapter, inside `adapters/fusion`: `uv sync`, `uv run ruff check`,
`uv run pytest`. SOLIDWORKS adapter, inside `adapters/solidworks`: the
`dotnet build` and `dotnet test` commands in its README (never the whole
`.sln` without SOLIDWORKS).

Then `grimble check` if you touched `design/`, and `frob check`.

## Ticket workflow (maintainers and agents)

1. Find work with `frob ticket doable`; claim it and work only inside
   the ticket's scope. Out-of-scope work you discover becomes a new
   ticket, not a drive-by fix.
2. Record evidence for the tests your change reaches, then gate with
   `frob check --ticket <handle>`.
3. User-visible changes get a changelog fragment in `changelog.d/`
   (`frob ticket fragment <handle>` writes the skeleton).
4. Keep every `frob:` directive (`frob:ticket`, `frob:todo`,
   `frob:doc`, `frob:tests`, `frob:invariant`, `frob:waive`) intact. A
   bare `TODO` comment fails the gate; use `frob:todo <ticket>`.

External contributors do not need to touch `tickets/`; the maintainer
files or closes the ticket when your PR lands.

## Rules every change follows

- ASCII only in every file; no emoji, smart quotes or em dashes.
- Commit format: `<type>(<scope>): <imperative summary, 72 chars max>`,
  then an optional body wrapped at 72 explaining why. Types: feat, fix,
  chore, refactor, test, docs, perf, ci, build. No trailing period, one
  logical change per commit, never amend a pushed commit, never skip
  hooks.
- No `Co-Authored-By` trailer or other attribution line in commits.
- Rust: every public item has a doc comment (`missing_docs` is on);
  `unsafe` is forbidden; fallible operations return typed `Result`s and
  panics are for programmer bugs; log through `tracing`, never
  `println!` outside the CLI's output layer.
- Python: pydantic v2 models, typani `Result[T, E]` for fallible
  operations, one-line docstrings on public symbols, module loggers
  instead of `print`.
- Update `docs/` in the same change as the code it describes.
- No new dependencies without discussion; open an issue first.
- Never read or write `.env` files or private keys.

## AI-assisted contributions

AI assistance is welcome under a strict policy: say in the PR which tool
you used and how, read and understand every line you submit, and expect
to explain it in review. Unreviewed generated diffs, boilerplate that
ignores this repository, sweeping changes without a prior ticket or
issue, and tests that only restate current behavior are not accepted.
Agents follow the workflow above exactly, stay in scope, run the gates
before claiming done, report results honestly, and never merge their own
work. An agent-authored PR still needs a named human author.

## Releases (maintainers)

Pushing a tag `cadetta-v<version>` (matching the workspace version) runs
`.github/workflows/release.yml`. Crates publish to crates.io from the
protected `crates-io` environment in dependency order; `cadetta`
publishes to PyPI from the protected `pypi` environment and
`cadetta-freecad` from `pypi-freecad`. All use trusted publishing. The
C ABI (`cadetta-capi`) ships as per-platform archives attached to the
GitHub release.

## Reporting bugs and proposing features

Use the issue forms under `.github/ISSUE_TEMPLATE`: bug report, feature
request, and spec question / ambiguity. Report vulnerabilities as
described in [SECURITY.md](SECURITY.md).

## License

By contributing to cadetta you agree that your contributions are
licensed under the project's MIT license (see [LICENSE](LICENSE)).
