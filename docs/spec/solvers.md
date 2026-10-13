# Solvers

A solver is an external program (Python, MATLAB, Octave, C++, anything)
that computes values cadetta cannot: FEA sizing, optimization, thermal
models. Cadetta never runs a solver during `check`; it runs them on
request, caches their results in a committed file, and tracks staleness
through the obligation graph.

## 1. Declaration

```
solver wall_sizing @"analysis/wall_sizing.py" {
  in  load = Frame.max_load, span = Frame.span
  out min_wall: Length                   // any kind or kind expression: `out q: Volume / Time`
  deps "analysis/material_db.csv"
}

model Frame {
  wall = 3mm
  span = 300mm
  max_load = 1200N
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
- An `in` expression must be quantity-valued: quantity literals, name
  paths and field reads, calls, indexing, unary minus, `+ - * / ^`, `as`
  and tolerances. A bare number as the whole input (or an operand of
  `+ -`) is Q001; a string, boolean, array, comparison, logic or range
  operator, `satisfies`, ISO class, `if`, `match` or block is S007. A bare
  number is a legal factor, exponent or call argument.
- The IR `Solver` carries a closure: the sorted paths its inputs read plus
  the script and `deps` files. Each output is `bound` until a driving use
  (section 2) marks it `driving`.
- All relative paths (the script, `deps`, result files) resolve against
  the directory of `cadetta.toml`, never the declaring file or the
  working directory.
- A solver is identified by its qualified path
  (`acme.rover::wall_sizing`). The command that runs it is
  configured, not written in source, keyed by that path
  (`[solvers."acme.rover::wall_sizing"] command = ["python",
  "{script}"]`), so source stays portable across machines and licenses
  and two packages or modules may declare solvers of the same name.

## 2. Bound by default, driving when written

Solver outputs are plain values; their use decides their role.

- **Bound** (default idiom, taught by `cadetta init` and the docs): a
  check compares a design value against the output. Re-running the
  analysis moves geometry only when the requirement is violated.
- **Driving**: `wall = round_to(wall_sizing.wall, 0.5mm)`. Fully
  supported. Every re-solve that changes the output creates a sync
  obligation for every bound CAD file downstream.

Evaluation rules (cadetta-solve `outputs`): a bound use is only compared
and never drives a value or changes the output's role; a driving use
marks the output `driving` and every value derived from it cites the
solver and output name. A stale result (section 4) still evaluates in
either role, carrying the S003 fact that names what changed (`inputs`,
`script`, or a dep path). An output with no recorded value is unsolved.

## 3. Protocol

No library is needed in any language: inputs and outputs are JSON, and
quantities travel as cadetta literal strings, parsed by cadetta's own
lexer.

Request (`cadetta.solve/1`):

```json
{ "schema": "cadetta.solve/1",
  "solver": "acme.rover::wall_sizing",
  "inputs": { "load": {"lit": "1200N",  "si": 1200.0},
              "span": {"lit": "300mm", "si": 0.3, "min": 0.2995, "max": 0.3005} } }
```

Response:

```json
{ "schema": "cadetta.solve/1",
  "version": "1.4.2",
  "outputs": { "min_wall": "3.2mm" },
  "diagnostics": [ {"severity": "info", "message": "converged in 14 iterations"} ] }
```

- `lit` is exact; `si` is a convenience float in SI base units. A
  toleranced input (`frame.span = 300mm [+-0.5mm]`) keeps `lit` as the
  nominal literal and adds `min` and `max` in SI base units; an
  untoleranced input has neither. `{lit, si, min, max}` is the whole
  field set.
- `version` is optional in the response; when present it enters the
  result digest (section 4).
- Outputs MUST be literals with units; a bare number is Q001 against the
  solver. A decimal literal is exact; there is no other float path.
  The Python decorator writes decimal strings rounded to `significant`
  digits (default 6, overridable per output), not `repr(float)`, which
  would trip M001 and Q006 at the consumer (bindings.md 3.3). Other
  languages must do the same.
- Output classification (decided in cadetta-solve): an output must be a
  JSON string that lexes as exactly one quantity literal. A JSON number,
  a bare-number string, or a string holding any other expression is Q001
  against the solver. A lexical error, a non-string value, bad JSON, or a
  missing or different `schema` is S005 (a typed protocol error).
- Two transports: `stdio` (request on stdin, response on stdout) and
  `file` (the command template receives `{in}` and `{out}` paths; for
  MATLAB: `["matlab", "-batch", "wall_sizing('{in}','{out}')"]`).
- Command template (decided in cadetta-solve): `command` is a list of
  arguments. `{script}`, `{in}` and `{out}` are the only placeholders,
  expanded inside each argument; `{in}` and `{out}` exist only under the
  `file` transport, and an unknown or unavailable placeholder is a
  configuration error (exit 2). The solver runs with the directory of
  `cadetta.toml` as its working directory. An optional `timeout` (whole
  seconds) kills the process and is a solver error (S005). Every solver
  error names the solver. Running a solver with no `[solvers."PATH"]`
  table is a usage error (exit 2).
- Exit status non-zero, malformed JSON, a missing output or a wrong kind
  are solver errors (S005) and leave the previous result in place.

## 4. Results and staleness

`cadetta solve [PATH-GLOB...] [--module PATH]` runs solvers and writes
`<stem>.out.json` beside the script (`drive_torque.py` writes
`drive_torque.out.json`): the response plus the digest of
every input value, the script and each dep. Its schema is
`cadetta.solve-result/1`: `{ "schema", "results": [ { "solver": path,
"input_digest", "script_digest", "dep_digests": {path: digest},
"version"?, "variant"?: "SLOT=ALT", "inputs": { name: { lit, si, min?, max? } },
"outputs": { name: literal }, "diagnostics": [] } ] }`,
sorted by solver path then variant. A solver's path is its declaration's
canonical path: `acme.drive::DriveModule.drive_torque` for a solver
declared as a member of `DriveModule`, `acme.drive::drive_torque` at file
scope. One file may hold several
solvers, keyed inside by solver path, so two solvers sharing a script
never collide. The result file is committed. `check` reads it; if any recorded digest differs from the
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

Solver result digests live only here: the lock file does not record
them (obligations.md 3), so the committed `.out.json` is the single
home of a solver's staleness evidence.

## 5. Trust

Solver outputs are untrusted data. A solver may declare checks on its
own outputs (`check positive: min_wall > 0mm`), and every check that reads an
output is evaluated like any other. A solver never writes source, CAD
files or the lock file.

## Language helpers

The Python package ships a solver decorator that hides the JSON and
checks kinds both ways (bindings.md 3.3). Other languages speak the
protocol directly; nothing in cadetta depends on a helper.
