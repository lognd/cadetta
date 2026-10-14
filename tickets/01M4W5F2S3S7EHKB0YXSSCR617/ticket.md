+++
id = "01M4W5F2S3S7EHKB0YXSSCR617"
title = "golden README: document runner rules"
type = "task"
category = "done"
outcome = "done"
priority = "medium"
points = 1
reporter = "lognd"
created = "2026-10-14T02:57:03Z"
updated = "2026-10-14T05:19:26Z"
scope = ["tests/golden/README.md"]
+++

Replace the "Running" section of tests/golden/README.md with:

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

Found while working ~J3392AZ (scope excluded tests/**).
