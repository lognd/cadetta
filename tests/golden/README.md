# Golden corpus

Realistic designs with their expected results. Each case is a small
project that exercises several spec documents at once. The conformance
corpus (`tests/conformance/<doc>/`) is a different thing: it holds one
must-fire and one must-not-fire fixture per rule.

The cases began as the pilots used to shape the specification (history
in `docs/history/`), and are kept current with the spec. The expected
results were evaluated by hand. They are trusted until the
implementation disagrees; when it does, either the implementation or
the hand evaluation is wrong, and the case says which.

## Layout

```
tests/golden/<NN-name>/
  README.md        what the case is and which spec sections it exercises
  cadetta.toml     the case's package (packages.md 1)
  src/             the design: src/lib.cdt and any modules (packages.md 2)
  analysis/        solver scripts and committed result files, when used
  bindings/        bindings files, when the case emits or reads CAD data
  input/           files the case reads (e.g. a legacy equations file)
  expected.toml    expected values, verdicts and diagnostics
  expected/        expected artifacts, compared byte for byte
```

## `expected.toml`

```toml
[case]
spec = "1.4"                  # spec draft the case is written against
status = "hand-evaluated"     # "confirmed" once the implementation reproduces it
exercises = ["stdlib.md 3", "geometry.md 5"]

# Evaluated values. Quantities are written as source literals and
# compared as exact quantities, so the unit used does not matter.
[[value]]
path = "acme.bracket::Mount/screws.clearance_hole"
nominal = "5.5mm"
min = "5.5mm"                 # optional: limits after tolerance
max = "5.5mm"
variant = "acme.rover::Rover/wheel=acme.rover::Colson2Series"   # optional
note = "Iso273 M5 medium"     # optional: how it was derived

# Check, fit, bound and require verdicts.
[[verdict]]
path = "acme.bracket::Mount.edge_ok"
result = "holds"              # holds | violates | undecided
conservative = false          # optional
variant = "..."               # optional

# Diagnostics. The list is closed for errors and warnings: an error or
# warning that the run produces and the list lacks is a failure.
# Info diagnostics are compared only when listed.
[[diagnostic]]
code = "C002"
path = "acme.bracket::Mount.edge_ok"
variant = "..."               # optional

# Artifacts. The command runs at the case root, and the file it writes
# must equal the expected file byte for byte.
[[artifact]]
command = "cadetta emit sw-equations"
produces = "build/solidworks/equations.txt"
expected = "expected/equations.txt"
# A line pattern (regular expression) whose matches are replaced by
# `<masked>` in both files before the compare; for digests that cannot
# be computed by hand.
mask = ['blake3:\S+']
```

Paths use the full canonical form (ir.md 2), including in `variant`.
Values are pinned at instance paths; a parameter-free component's
definition path is its instance path (ir.md 2).

## Things hand evaluation cannot produce

- **Digests.** A committed file that records digests (a solver result,
  a lock) carries `blake3:TBD`, and artifacts mask their digest lines.
  When the implementation first confirms a case, it writes the real
  digests, and the case's `status` becomes `confirmed`.
- **Standard-library numbers.** The spec names `std`'s tables and
  columns; the numbers are in `std` itself, cited to their standards.
  Cases use the published figures (ISO 273, ISO 4762 and so on) and say
  so in a comment at the top of `expected.toml`. If `std` disagrees
  with one, check the standard.

## Running

The runner is crates/cadetta-cli/tests/golden (cargo test -p cadetta --test golden). For each case it
copies the case to a scratch directory, runs `cadetta check --format json` (diagnostics) and
`cadetta ir` (values and verdicts, which `check` does not report), then every listed artifact command
(a trailing `> file` captures stdout), and compares with expected.toml and expected/.

- Values compare as exact quantities in any unit; min and max are the nominal plus the lowest and
  highest tolerance deviations. holds/violates/undecided correspond to the IR's pass/fail/unknown.
- Diagnostics are closed for live (non-waived) errors and warnings; info only when listed.
- An expectation the implementation cannot yet produce is reported as unevaluable, not as a mismatch.
- A mismatch names its suspect: the implementation when the run is not clean (the IR fails to build
  or check reports a live error), the hand evaluation when the run is clean.
- status = "confirmed": any mismatch or unevaluable expectation fails the test (a regression).
- status = "hand-evaluated": findings are reported but do not fail the test, unless
  CADETTA_GOLDEN_STRICT=1 is set.
- A case whose spec differs from the current draft (the `Status: draft X.Y` line of
  docs/spec/README.md) always fails until it is ported.
- CADETTA_GOLDEN_CONFIRM=1 (explicit opt-in) runs `cadetta solve` on a scratch copy of each
  hand-evaluated, current-spec case with no mismatch and no unevaluable expectation, writes the real
  digests over every `blake3:TBD` file (only when the files differ by digests alone), and sets
  status = "confirmed".
- The per-case summary goes to stderr and to $CARGO_TARGET_TMPDIR/golden-report.txt.
