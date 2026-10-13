# Configuration and command line

## 1. Project model

1. A project is a **package** or a **workspace** (packages.md 1): the
   directory holding a `cadetta.toml` with a `[package]` or a
   `[workspace]` table. A package has one `cadetta.lock`, one `build/`
   and one set of adapters; workspace members share one lock and one
   `build/` (packages.md 1). Relative paths in `cadetta.toml`, in
   citations (`@"file"`), in solver scripts and in bindings resolve
   against the package directory.
2. A file's module is its path under the package's `src` (packages.md
   2). The IR holds the package, its
   `modules[]` and its `dependencies[]` (ir.md).
3. Matches under `src` are sorted by path before compile so digests are
   deterministic. A package whose `src` directory is missing or holds no
   `.cdt` file, or a workspace member or path dependency without a
   `cadetta.toml`, is E005.
4. A component name defined twice in one module is E003, with both
   spans.
5. Reuse: shared parts live in a package that other packages list under
   `[dependencies]` (packages.md 1, 4). Dependencies are pinned in
   `cadetta.lock` and re-resolved by `cadetta update`.
6. Root discovery: from `--cwd` (default the current directory) walk up
   to the nearest `cadetta.toml` with a `[package]` or `[workspace]`
   table; `--config FILE` names one explicitly. If none is found, exit
   2. In a workspace, a command run inside a member scopes to that
   member unless run at the workspace root. `--config FILE` also
   starts the walk at FILE's directory, so FILE need not itself carry a
   project table (the nearest one at or above it wins). The Python `Project.open`
   follows the same rule.

## 2. `cadetta.toml`

`cadetta init` writes a complete file with every key at its default and
a comment per key. Defaults are shown below.

```toml
[package]
name = "acme.rover"              # lower.dotted (N001); default: directory name, normalised
version = "0.1.0"                # semver
src = "src"                      # module root; default "src"
as_of = "2026-10-13"             # fixed date for expiry; default today

[dependencies]
"acme.shop" = { path = "../shop", version = "1" }   # packages.md 1

# A workspace root has [workspace] instead of [package]:
# [workspace]
# members = ["rover", "shop"]

[defaults]
# Project-wide parameter defaults (qualified path); lowest precedence.
FastenerPolicy = "acme.shop::AcmeShop"

[units]
# Display unit per kind; authored units win when unset.
display = { Length = "mm", Angle = "deg", Mass = "kg" }

[emit]
# Tolerance point written to CAD: nominal | mid | mmc | lmc.
point = "nominal"

[lint]
fail_on = "error"                # error | warning | info | never
# Max significant decimals in the display unit before M001 fires.
precision = { Length = 2, Angle = 1 }
# D001 fires at full severity only from this many significant digits.
d001_min_digits = 3
# Round numbers D001 never reports.
common_values = ["1mm", "2mm", "5mm", "10mm", "0.5in", "1in"]
# Per-code severity overrides; can only raise a required_for floor.
severity = { N002 = "info" }
# W005 fires for waivers and eol dates this close to expiry.
expiry_warn_days = 30

[obligations.roles.source]
covers = ["ots:*"]               # selectors, obligations.md 4
required_for = "check"           # check | release; unset = not gating
members = ["a@x.org"]            # optional: who may attest
distinct_from_author = false

[obligations.kinds.sync]
required_for = "release"         # gate sync or solver obligations

[solvers."acme.rover::wall_sizing"]
command = ["python", "{script}"]
transport = "stdio"              # stdio | file

[[adapter]]
name = "sw-equations"            # built-in
id = "rover-sw"                  # default: name; required if name repeats
out = "build/solidworks/rover-sw.txt"
bindings = "bindings/rover-sw.toml"   # default bindings/<id>.toml
modules = ["drive"]              # module path prefixes; default all
paths = []                       # canonical-path globs; default all
prefix = ""                      # prepended to created native names
native_unit = "mm"               # unit for unitless native values (adapters.md 3)
precision = 6                    # decimals; default from the emitter spec (adapters.md 5)

[[adapter]]
name = "solidworks"              # external, Windows only
command = ["cadetta-solidworks"]
version = "0.1"
files = ["**/*.SLDPRT", "**/*.SLDASM"]
save = "after-sync"              # after-sync | never (adapters.md 2.2)
native_unit = "mm"
```

Key notes:

- `[defaults]` precedence, highest first: the component's own
  `#![default]`, the file's `#![default]`, project `[defaults]`, std
  (language.md 3.11, stdlib.md 4).
- `fail_on` is the lowest severity that makes a command exit 1; `never`
  makes diagnostics informational.
- Severity precedence: a kind's `required_for` sets the floor of its
  diagnostic; `[lint] severity` can only raise it.
- `command`, `version` and `files` are for external adapters; built-ins
  take `out`. `version` must match `initialize` (adapters.md 5).
- Solvers are keyed by qualified path. Several solvers may share one
  script; `<stem>.out.json` then holds one entry per solver path
  (solvers.md 4).
- `as_of` is overridden by `--as-of`.
- Loading is strict: an unknown key anywhere is a configuration error
  (exit 2) that names it, so the retired `[project]` table and its
  `sources` and `roots` keys are rejected. A file has exactly one of
  `[package]` and `[workspace]`. `precision` values and `d001_min_digits`
  are integers (`d001_min_digits >= 1`); `common_values` stay quantity
  text until sema reads them. `transport` defaults to `stdio`, `save`
  to `after-sync`, a role's `covers` must be non-empty, and a repeated
  adapter `name` needs distinct `id`s.
- `[defaults]` applies per package. `[package]`, `[dependencies]` and
  `[workspace]` are defined in packages.md 1; I004 applies only to the
  roots of the package being checked (packages.md 3).

## 3. Commands

Exit codes: `0` ok; `1` diagnostics at or above `fail_on`, or a stale or
blocked verdict (below); `2` usage, I/O or configuration error, adapter
unavailable (R008 during `sync`), unknown path or selector.

| Command | Does | Reads / writes | Exit 1 when | Output schema |
|---|---|---|---|---|
| `init [--adapter solidworks\|freecad\|sw-equations] [--bare]` | Write `cadetta.toml` with `[package]`, `src/lib.cdt` and the adapter block. | writes missing files only | never | - |
| `fmt [--check]` | Canonical formatting (language.md 7). | `src` files | `--check` finds a diff | - |
| `check [--release] [--as-of DATE] [--watch] [--module PATH] [GLOB...]` | Compile, evaluate, lint, obligations; with `read` adapters, drift lints. | reads all | diagnostics >= `fail_on` | `check.json` |
| `solve [GLOB...] [--module PATH] [--variants]` | Run solvers, refresh result files (solvers.md). | writes `<stem>.out.json` | diagnostics >= `fail_on` (S005 is one) | `solve-report.json` |
| `update [NAME]` | Re-resolve dependencies (all, or NAME) and write the `package` records of the lock (packages.md 1). | writes `cadetta.lock` | E012, E013 | `update.json` |
| `why PATH` | Upstream derivation. | reads | never | `why.json` |
| `why --stale OBLIGATION` | Nearest changed upstream node, old and new values. | reads, lock | never | `why.json` |
| `impact PATH` | Downstream effects of changing PATH. | reads | never | `impact.json` |
| `impact --since REV` / `--worktree` | Diff of two evaluations: changed nodes, stale obligations, flipped verdicts, CAD files to re-sync. | reads, git | never | `impact.json` |
| `impact --swap SLOT ALT` | Swap preview (obligations.md 5.1). `SLOT` is a canonical path (`acme.rover::Rover/wheel`), `ALT` a qualified component name. | reads | verdict `blocked` | `swap.json` |
| `swap SLOT ALT --reason "..."` | Apply the swap preview's suggested edit to source and write a `swap` lock record (obligations.md 3). | writes source, `cadetta.lock` | verdict `blocked` (nothing written) | `swap.json` |
| `compare IFACE` | Implementers of an interface side by side. | reads | never | `compare.json` |
| `obligations [--stale] [--role R] [--release] [--as-of DATE]` | Open and stale obligations. | reads, lock | any obligation required for the gate is open or stale | `obligations.json` |
| `ack SELECTOR --role R --reason "..." [--baseline] [--release] [--dry-run] [--as EMAIL]` | Attest at current digests; `--baseline` records reason `baseline` (obligations.md 4); `waiver:GLOB` records reviewed waivers' digests. | writes `cadetta.lock` | never | `ack.json` |
| `lock merge\|verify\|tidy` | Merge driver; list attestations a merge invalidated; drop superseded records. | `cadetta.lock` | `verify` finds invalid records | `lock.json` |
| `cite pin NAME` | Download a URL citation into `cites/` and rewrite the `cite` to the local path. | writes `cites/`, source | never (errors exit 2) | - |
| `ir [--observed ADAPTER] [--assume-unit U]` | Print design or observed IR as `cadetta.ir/1`. | reads | never | `ir.json` |
| `emit [ADAPTER...] [--module PATH] [GLOB...]` | Run `emit` for configured adapters. | writes adapter outputs | Q004, R013 | `emit.json` |
| `import ADAPTER FILE... [--assume-unit U]` | Propose bindings and scaffold (adapters.md 7). | writes `build/import/` | never | `import.json` |
| `import --accept` | Write bindings and `.cdt` scaffold from the proposal. | writes `bindings/`, `cad/imported/`, lock | never | `import.json` |
| `import ADAPTER FILE... --update [--write]` | Diff of `.cdt` literal edits from observed values; `--write` applies. | writes sources with `--write` | never | `import.json` |
| `sync [--dry-run] [--only GLOB] [--force-design] [--no-regen] [--module PATH] [ADAPTER-ID...]` | The three-way loop (adapters.md 2). | writes CAD files, outputs, lock | R001, R004, R009, R010, Q004, R013 | `sync.json` |
| `explain CODE` | Print the catalog entry for a diagnostic. | - | never | `explain.json` |
| `units [KIND]` | Print the unit and kind tables. | - | never | `units.json` |

`check`, `emit`, `sync` and `solve` accept `--module PATH` (a module
path prefix, e.g. `--module drive`) and canonical-path globs to scope a
run. `cadetta lsp` is deferred (a
ticket); in 0.1.0 the editor integration is `check --format json
--watch` (5).

Scoping (decided, 0.1.0): a node is in scope when it matches any
`--module` prefix (if given) and any glob (if given); with neither,
everything is. A module prefix is compared segment-wise against the
canonical path, relative to the package or fully qualified, and accepts
`/` or `::` (`drive` matches `acme.rover::drive::motor::X` but not
`acme.rover::driveline::X`). Globs use the obligations.md 2.2 grammar.
A scope matching no node is not an error for these commands.

### 3.1 `sync` output

Text: one line per locator,
`push|skip|drift|converged|fail  path  locator  old -> new`, then a
summary and a per-file regen status. JSON (`sync.json`): the same
records plus diagnostics. "Changed" means the emit value differs from
the lock base. `--dry-run` prints every push, including each native
expression it would replace (R006), and writes nothing. `--only GLOB`
limits to matching canonical paths; `--no-regen` pushes without
rebuilding (the lock is then not updated).

### 3.2 `init`

Creates missing files only; it never rewrites an existing file, prints
one info line per skipped file and exits 0. `--adapter solidworks`
writes the `[[adapter]]` block (command, `files` globs, `save`,
`native_unit`) and prints the install line
(`pip install "cadetta[solidworks]"`, run in the Windows interpreter).
`--bare` omits the hello-world item in `src/lib.cdt`. `init` writes
`[package]` and `src/lib.cdt`; the package name defaults to the
directory name, normalised, and is echoed.

Decided (0.1.0): `fmt` rewrites every `.cdt` file of the scoped
packages in place and prints `reformatted FILE`; `fmt --check` writes
nothing, prints `would reformat FILE` per differing file and exits 1. A
file with syntax errors cannot be formatted: it is reported on stderr and
the run exits 2. `init --adapter freecad` writes an external
`cadetta-freecad` block (`files = ["**/*.FCStd"]`); `sw-equations`
writes the built-in block with `out = "build/solidworks/equations.txt"`.
The hello-world item is `model Hello { size: Length = 10mm }`.

### 3.3 Selectors

`SELECTOR` is `KIND:GLOB[,KIND:GLOB...]` over obligation ids
(`<kind>:<canonical path>`, obligations.md 4). A selector matching
nothing is exit 2.

Decided (0.1.0): `cite pin NAME` accepts only a `http(s)` string
citation declared once in the scoped packages. It stores the bytes as
`cites/<first 16 hex of blake3>-<file name>` (package-relative; same
bytes, same path) and rewrites the `cite` string to that path. A path,
a standard or an unknown or ambiguous NAME is a usage error (exit 2).

## 4. Global flags

| Flag | Meaning |
|---|---|
| `--format text\|short\|github\|sarif\|json` | Output format (5); env `CADETTA_FORMAT`. |
| `-v` / `-vv` / `-vvv` | More logging on stderr: info, debug, trace (warn at 0); env `CADETTA_LOG` (tracing env-filter directives) overrides it, and an invalid filter is a usage error (exit 2). |
| `--cwd DIR` | Start root discovery here (1). |
| `--config FILE` | Use this `cadetta.toml`. |

`--as-of DATE` (`check`, `obligations`; default today or `[package]
as_of`) fixes the date for waiver and `eol` expiry (W001, W005, L001);
the JSON report header records it so a historic result is reproducible.

## 5. Output formats

stdout carries the command's output; stderr carries logs. The `cadetta-log` crate is the only owner of subscriber setup (frob.toml `[invariants]` and clippy `print_stdout`/`print_stderr` enforce it). The default is
`text` everywhere, whether or not stdout is a terminal; JSON is chosen
only by `--format json` or `CADETTA_FORMAT`.

| Format | Shape |
|---|---|
| `text` | rustc style: `severity[CODE]: message`, then `--> file:line:col`, a source excerpt, and `= note:`, `= help:`, `= path:` lines (lints.md 1.2) |
| `short` | one line per diagnostic, grep and problem-matcher friendly |
| `github` | workflow annotations |
| `sarif` | SARIF 2.1 for code scanning |
| `json` | the command's schema (3); diagnostics are `cadetta.diag/1` records (lints.md 1.1) |

`check --format json --watch` re-runs on source change and emits one
`cadetta.diag/1` record per line (newline-delimited), sorted by (file,
line, code) within a run; it is the editor problem-matcher integration.
Schemas are generated from the Rust types and committed in
`docs/schemas/`.

## 6. Committed artifacts

| Artifact | Location | Written by | Committed |
|---|---|---|---|
| `cadetta.toml` | package or workspace root | `init`, by hand | yes |
| `cadetta.lock` | package or workspace root | `ack`, `sync`, `import --accept`, `lock` | yes; `.gitattributes`: `cadetta.lock merge=cadetta` |
| bindings | `bindings/<id>.toml` (or `[[adapter]] bindings`) | `import --accept`, by hand | yes |
| solver results | `<stem>.out.json`, beside the script | `solve` | yes |
| imported scaffold | `cad/imported/<File>.cdt` | `import --accept` | yes |
| pinned citations | `cites/` | `cite pin` | yes (or LFS) |
| JSON schemas | `docs/schemas/` | generated from the Rust types | yes |
| emitted files, proposals | `build/` (`build/import/`, adapter `out` defaults) | `emit`, `sync`, `import` | no (gitignored) |
