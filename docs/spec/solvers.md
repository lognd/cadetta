# Solvers

A solver is an external program (Python, MATLAB, Octave, C++, anything)
that computes values cadetta cannot: FEA sizing, optimization, thermal
models. Cadetta never runs a solver during `check`; it runs them on
request, caches their results in a committed file, and tracks staleness
through the obligation graph.

## 1. Declaration

```
solver wall_sizing @"analysis/wall_sizing.py" {
  in  load = frame.max_load, span = frame.span
  out min_wall: Length                   // any kind or kind expression: `out q: Volume / Time`
  deps "analysis/material_db.csv"
}

model Frame {
  wall = 3mm
  check wall_ok: wall >= wall_sizing.min_wall          // bound use (default idiom)
}
```

- `in` binds named inputs to expressions; `out` declares outputs with
  kinds; `deps` lists extra files whose digests count as inputs.
- Input expressions resolve where the solver is declared. At file
  scope they name components (`Frame.span`); a solver declared as a
  member of an assembly resolves like any member, so instances and
  roll-ups are in scope (`motor.stall_torque`, `mass?(self)`).
- The citation names the script; its digest is an input.
- The command that runs it is configured, not written in source
  (`[solvers.wall_sizing] command = ["python", "{script}"]`), so
  source stays portable across machines and licenses.

## 2. Bound by default, driving when written

Solver outputs are plain values; their use decides their role.

- **Bound** (default idiom, taught by `cadetta init` and the docs): a
  check compares a design value against the output. Re-running the
  analysis moves geometry only when the requirement is violated.
- **Driving**: `wall = round_to(wall_sizing.wall, 0.5mm)`. Fully
  supported. Every re-solve that changes the output creates a sync
  obligation for every bound CAD file downstream.

## 3. Protocol

No library is needed in any language: inputs and outputs are JSON, and
quantities travel as cadetta literal strings, parsed by cadetta's own
lexer.

Request (`cadetta.solve/1`):

```json
{ "schema": "cadetta.solve/1",
  "solver": "wall_sizing",
  "inputs": { "load": {"lit": "1200N",  "si": 1200.0},
              "span": {"lit": "300mm", "si": 0.3} } }
```

Response:

```json
{ "schema": "cadetta.solve/1",
  "outputs": { "min_wall": "3.2mm" },
  "diagnostics": [ {"severity": "info", "message": "converged in 14 iterations"} ] }
```

- `lit` is exact; `si` is a convenience float in SI base units.
- Outputs MUST be literals with units; a bare number is Q001 against the
  solver. A decimal literal is exact; there is no other float path.
- Two transports: `stdio` (request on stdin, response on stdout) and
  `file` (the command template receives `{in}` and `{out}` paths; for
  MATLAB: `["matlab", "-batch", "wall_sizing('{in}','{out}')"]`).
- Exit status non-zero, malformed JSON, a missing output or a wrong kind
  are solver errors (S005) and leave the previous result in place.

## 4. Results and staleness

`cadetta solve [NAME...]` runs solvers and writes
`<script>.out.json` beside the script: the response plus the digest of
every input value, the script and each dep. The result file is
committed. `check` reads it; if any recorded digest differs from the
current one, S003 (`solver-stale`) fires and the stale outputs still
evaluate, marked stale, so the rest of the design keeps checking.

### 4.1 Solvers under variant evaluation

Results are keyed by the digest of their input values. When variant
evaluation (language.md 3.2) changes a solver's inputs, the variant
needs its own result: `cadetta solve --variants` runs the solver once
per distinct input set and stores each result in the same file. If a
variant's result is missing, its outputs are unknown, never borrowed
from the primary: checks and requires reading them are `undecided`
under that variant, with S006 (`solver-unsolved-variant`), and
`impact --swap` says which solve to run.

## 5. Trust

Solver outputs are untrusted data. A solver may declare checks on its
own outputs (`check positive: min_wall > 0mm`), and every check that reads an
output is evaluated like any other. A solver never writes source, CAD
files or the lock file.

## Language helpers (non-normative)

Thin helpers that hide the JSON may be published later, Python first.
Nothing in cadetta depends on them.
