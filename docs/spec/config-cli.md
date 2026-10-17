# Configuration and command line

## 1. Project model

1. A project is a **package** or a **workspace** (packages.md 1): the
   directory holding a `cadetta.toml` with a `[package]` or a
   `[workspace]` table. A package has one `cadetta.lock`, one `build/`
   and one set of adapters; workspace members share one lock and one
   `build/` (packages.md 1). Relative paths in `cadetta.toml`, in a
   citation's `file`, in solver scripts and in bindings resolve
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
timeout = 60                     # seconds; optional, unset = no limit

[[adapter]]
name = "sw-equations"            # built-in
id = "rover-sw"                  # default: name; required if name repeats
out = "build/solidworks/rover-sw.txt"
bindings = "bindings/rover-sw.toml"   # default bindings/<id>.toml
modules = ["drive"]              # module path prefixes; default all
paths = []                       # canonical-path globs; default all
prefix = ""                      # prepended to created native names
bind = "explicit"                # explicit | scope: also bind every in-scope path to its created global (adapters.md 3.5)
native_unit = "mm"               # unit for unitless native values (adapters.md 3)
precision = 6                    # decimals; default from the emitter spec (adapters.md 5)

[[adapter]]
name = "solidworks"              # external, Windows only
command = ["cadetta-solidworks"]
version = "0.1"
timeout = 60                     # seconds one call may stay unanswered before the adapter is killed; default 60 (adapters.md 5)
files = ["**/*.SLDPRT", "**/*.SLDASM"]
save = "after-sync"              # after-sync | never (adapters.md 2.2)
datums = "verify"                # verify | create: create and drive datum coordinate systems (adapters.md 3.4)
tolerances = "drive"             # drive | ignore: write dimension tolerances (adapters.md 3.6)
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
- `bind` applies to every adapter. `datums` and `tolerances` apply to
  adapters whose `initialize` features include `create_frames` and
  `tolerances`; elsewhere they are accepted and have no effect.
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
- `[defaults]` applies per package: each package's table is read by the
  evaluator as the project layer, and a value may name a component of a
  dependency (`acme.shop::AcmeShop`). `[package]`, `[dependencies]` and
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
| `solve [GLOB...] [--module PATH] [--variants]` | Run solvers, refresh result files (solvers.md). | writes `<stem>.out.json` | diagnostics >= `fail_on` (S005 is one) | `solve-report.json` (`cadetta.solve-report/1`: per-solver solved, fresh or failed, plus a `diagnostics` list with one error per failed solver; config errors, exit `2`, are not diagnostics) |
| `update [NAME]` | Re-resolve dependencies (all, or NAME) and write the `package` records of the lock (packages.md 1). | writes `cadetta.lock` | E012, E013 | `update.json` |
| `lock verify [--format json]` | Check the lock is well formed, sorted, unique and policy-conformant; list merge-invalidated attestations (obligations.md 3). Text prints `cadetta.lock: clean` or one line per finding; JSON is `cadetta.lock/1`. A missing lock is clean; an unparsable lock exits `2`. | reads, lock | any finding (exit `1`) | `lock.json` |
| `why PATH` | Upstream derivation. | reads | never | `why.json` |
| `why --stale OBLIGATION` | Nearest changed upstream node, old and new values. | reads, lock | never | `why.json` |
| `impact PATH` | Downstream effects of changing PATH. | reads | never | `impact.json` |
| `impact --since REV` / `--worktree` | Diff of two evaluations: changed nodes, stale obligations, flipped verdicts, CAD files to re-sync. | reads, git | never | `impact-since.json` |
| `impact --swap SLOT ALT` | Swap preview (obligations.md 5.1). `SLOT` is a canonical path (`acme.rover::Rover/wheel`), `ALT` a qualified component name. | reads | verdict `blocked` | `swap.json` |
| `swap SLOT ALT --reason "..."` | Apply the swap preview's suggested edit to source and write a `swap` lock record (obligations.md 3). | writes source, `cadetta.lock` | verdict `blocked` (nothing written) | `swap.json` |
| `compare IFACE` | Implementers of an interface side by side. | reads | never | `compare.json` |
| `obligations [--stale] [--role R] [--release] [--as-of DATE]` | Open and stale obligations: every `check`, every `ots` component and every cite item an `ots` reaches (`citation:PATH`). | reads, lock | any obligation required for the gate is open or stale | `obligations.json` |
| `ack SELECTOR --role R --reason "..." [--baseline] [--release] [--dry-run] [--as EMAIL]` | Attest at current digests; `--baseline` records reason `baseline` (obligations.md 4); `waiver:GLOB` records reviewed waivers' digests. | writes `cadetta.lock` | never | `ack.json` |
| `lock merge\|verify\|tidy` | Merge driver; list attestations a merge invalidated; drop superseded records. | `cadetta.lock` | `verify` finds invalid records | `lock.json` |
| `cite pin NAME [--as-of DATE]` | Download a citation's `url` into `cites/` and add its `file` field. | writes `cites/`, source | never (errors exit 2) | - |
| `cite list [--format ieee\|bibtex\|json] [--module PATH] [GLOB...]` | The reference list of the citations the scoped design reaches (language.md 3.9.1). | reads | never | `cite-list.json` |
| `names [ADAPTER-ID] [--module PATH] [GLOB...]` | Every in-scope bindable path with its kind, emit value, tolerance, citation, doc, created native name (adapters.md 3.1) and binding state (explicit, implicit, unbound). | reads; with a `read` adapter, reads CAD | R013 | `names.json` |
| `bind PATH --adapter ID --locator L --kind K [--mode M] [--native-unit U] [--for ALT] [--config C] [--sign -1] [--component C] [--tolerance drive\|ignore]` / `bind --remove PATH --adapter ID --locator L` | Add or remove one explicit locator in `bindings/<id>.toml`, validated and written in canonical order. | writes the bindings file | never (errors exit 2) | `bind.json` |
| `ir [--observed ADAPTER] [--assume-unit U]` | Print design or observed IR as `cadetta.ir/1`. | reads | never | `ir.json` |
| `emit [ADAPTER...] [--module PATH] [GLOB...]` | Run `emit` for configured adapters. | writes adapter outputs; `emit.json` also lists `active_alternates` (slot path to its primary, adapters.md 3.3) when the package has interface-typed slots | Q004, R013, R014, R015 | `emit.json` |
| `import ADAPTER FILE... [--assume-unit U]` | Propose bindings and scaffold (adapters.md 7). | writes `build/import/` | never | `import.json` |
| `import --accept` | Write bindings and `.cdt` scaffold from the proposal. | writes `bindings/`, `cad/imported/`, lock | never | `import.json` |
| `import ADAPTER FILE... --update [--write]` | Diff of `.cdt` literal edits from observed values; `--write` applies. | writes sources with `--write` | never | `import.json` |
| `sync [--dry-run] [--only GLOB] [--force-design] [--no-regen] [--module PATH] [ADAPTER-ID...]` | The three-way loop (adapters.md 2), then geometry verification (R001) and, under `datums = "create"`, datum frame creation (adapters.md 3.4). | writes CAD files, outputs, lock | R001, R004, R009, R010, Q004, R013, R014, R015, R016 | `sync.json` |
| `explain CODE` | Print the catalog entry for a diagnostic. | - | never | `explain.json` |
| `units [KIND]` | Print the unit and kind tables. | - | never | `units.json` |
| `frames [--native-unit U] [--precision N] [--module PATH] [GLOB...]` | Print the composed datum frames and placement instance frames as 3x4 transforms. | - | never | `frames.json` |
| `display VALUE SPEC` | Print one quantity literal through the display formatter (quantities.md 6). | - | never (errors exit 2) | `display.json` |
| `lsp` | Language server over stdio: diagnostics, formatting, hover, code lens, definition (3.8). | reads; writes nothing | never (exits 0 on `exit` after `shutdown`, 1 otherwise) | LSP 3.17 |

`check`, `emit`, `sync` and `solve` accept `--module PATH` (a module
path prefix, e.g. `--module drive`) and canonical-path globs to scope a
run. Editors integrate through `cadetta lsp` (3.8); `check --format
short` and `check --format json --watch` (5) remain for problem matchers
and scripts.

Scoping (decided, 0.1.0): a node is in scope when it matches any
`--module` prefix (if given) and any glob (if given); with neither,
everything is. A module prefix is compared segment-wise against the
canonical path, relative to the package or fully qualified, and accepts
`/` or `::` (`drive` matches `acme.rover::drive::motor::X` but not
`acme.rover::driveline::X`). A dotted package name matches on dot
boundaries (`acme.gear` matches packages `acme.gear` and `acme.gear.sub`,
not `acme.gearbox`). The `[[adapter]] modules` list (adapters.md 3.2)
uses this same form. Globs use the obligations.md 2.2 grammar.

`check` pipeline and exit rules (decided, 0.1.0): `check` parses every
module (a lexical error E001 stops the run before evaluation, so later
stages never see a broken tree), resolves, evaluates, kind-checks,
lowers each in-scope package to IR and lints the IR; the `std` package
is embedded in the binary (a `$CADETTA_STD` directory holding a std
package overrides the embedded copy) and its own findings are not reported. A diagnostic's `path` is the
canonical path of the member it concerns (for a check verdict, the
check's path) and its `location` the project-relative file with 1-based
line and column. The file is the one of the module the finding was
raised in: resolve, evaluation and dependency diagnostics carry their
package and module path, so a finding is never attributed to another
file by span position, and one raised in `std` is not reported. A
diagnostic without a path is dropped from a scoped
run (`--module` or GLOB). Open and stale obligations become O002 and
O001; a row the role policy gates (`required_for = "check"`, or
`"release"` with `--release`) is an error whatever `[lint] severity`
says, the others keep their catalog severity. An obligation no role
covers is not raised (nothing could discharge it). Drift lints run once
per configured adapter that can read (an external adapter with a
`command`); emit-only built-ins are skipped. The exit code is 1 when an
unwaived diagnostic reaches `[lint] fail_on` and 0 otherwise, so
`fail_on = "never"` always exits 0; a malformed flag, scope or `--as-of`
is a usage error (2). `--watch` is the ndjson loop of section 5 (it requires
`--format json`; the first run emits every record, later runs emit only
records not in the previous run, so an unchanged run prints nothing).
A scope matching no node is not an error for these commands.

### 3.1 `sync` output

Text: one line per locator,
`push|skip|drift|converged|fail  path  locator  old -> new`, then a
summary and a per-file regen status. JSON (`sync.json`): the same
records plus diagnostics, each carrying `code`, `severity` (`info` for O004,
`error` for the blocking codes, else `warning`) and `message`. "Changed" means the emit value differs from
the lock base. `--dry-run` prints every push, including each native
expression it would replace (R006), and writes nothing. `--only GLOB`
limits to matching canonical paths; `--no-regen` pushes without
rebuilding (the lock is then not updated).

Implementation notes (`cadetta sync`, schema `docs/schemas/sync.json`):

- The adapter is read first (`read` over its `files` globs); the entry
  file of a locator is the observed member whose native name equals it
  (first match). A locator missing from the CAD files raises R003 and is
  skipped.
- Values are compared as normalized `{number}{unit}` text (trailing zeros
  trimmed, missing unit defaulted to the locator's native unit). Differing
  units compare textually; unit conversion and tolerance sync are not yet
  done.
- No Push record: the adapter is shut down, nothing is opened, rebuilt or
  saved; converged values still refresh the lock. With pushes the session
  is opened on the pushed files only, each file is rebuilt (unless
  `--no-regen`, reported `skipped`), read back and compared; a mismatch is
  `fail`. `save = "after-sync"` saves only when nothing failed and
  `--no-regen` is absent, recording `synced file=` digests.
- An adapter without `drive` (including every built-in): `emit` runs for
  `sw-equations` and the printed `manual` step says how to relink and
  rebuild by hand; `--dry-run` prints the step only. Nothing is classified.
- Exit 1: R001, R004, R009, R010, Q004, R013-R016 or any `fail`; exit 2:
  usage, IO, config, unknown adapter id, unavailable adapter (R008).

### 3.3 `ir`, `explain`, `units`, `display`

Decided (0.1.0): `explain CODE` takes the code case-insensitively and
prints the lints.md 3 catalog row (code, name, default severity, fixed or
configurable, meaning); `--format json` prints `explain.json`
(`cadetta.explain/1`); an unknown code exits 2. `units [KIND]` prints the
built-in kinds, each with its dimension, angle exponent, atom units (exact
SI factor `r * pi^k`) and compound units; `units.json` is
`cadetta.units/1`; an unknown KIND exits 2. `ir` prints one package's IR
as pretty `cadetta.ir/1` JSON (`cadetta.ir.1.json`) and exits 2 when the
run scopes to several workspace members; `--observed ADAPTER` names a
configured `[[adapter]]` (by `name` or `id`, else exit 2) and prints the
`observed` flavour; `--assume-unit U` must parse as a unit.
`display VALUE SPEC` takes a quantity literal and a display spec
(`in/32`, `ft_in`, `mm`, `%`), prints the formatted text and a newline, or
`display.json` (`cadetta.display/1`: `value`, `spec`, `text`) with
`--format json`; a malformed literal, spec or dimension mismatch exits 2.

`frames` composes each in-scope datum (frame in its own component) and each placement (the frame its anchor drives to) through the same composer `sync` verifies with, and prints 3x4 transforms in the native unit (`--native-unit`, default `mm`; `--precision`, default 6). `frames.json` is `cadetta.frames/1`; a frame that cannot be composed has a null transform; an unknown unit or flag exits 2.

### 3.2 `init`

Creates missing files only; it never rewrites an existing file, prints
one info line per skipped file and exits 0. `--adapter solidworks`
writes the `[[adapter]]` block (command, `files` globs, `save`,
`native_unit`) and prints where to get the SOLIDWORKS adapter and
add-in installer (solidworks.md 1).
`--bare` omits the hello-world item in `src/lib.cdt`. `init` writes
`[package]` and `src/lib.cdt`; the package name defaults to the
directory name, normalised, and is echoed.

Decided (0.1.0): `fmt` rewrites every `.cdt` file of the scoped
packages in place and prints `reformatted FILE`; `fmt --check` writes
nothing, prints `would reformat FILE` per differing file and exits 1. A
file with syntax errors cannot be formatted: it is reported on stderr and
the run exits 2. `init --adapter freecad` writes an external
`cadetta-freecad` block (`files = ["**/*.FCStd"]`); `sw-equations`
writes the built-in block with `out = "build/solidworks/equations.txt"`
and `bind = "scope"`, since an equations file is useful only with
globals to link (solidworks.md 2.4).
The hello-world item is `model Hello { size: Length = 10mm }`.

### 3.3 Selectors

`SELECTOR` is `KIND:GLOB[,KIND:GLOB...]` over obligation ids
(`<kind>:<canonical path>`, obligations.md 4). A selector matching
nothing is exit 2.

Decided (0.1.0): `cite pin NAME` takes a `cite` item's path (relative
or qualified) declared in the scoped packages, which must have a `url`.
It stores the bytes as `cites/<first 16 hex of blake3>-<file name>`
(package-relative; same bytes, same path), sets the item's `file` field
to that path (replacing an earlier pinned path), keeps `url`, and adds
`accessed` with the run's date (`--as-of`, else today) when the item has
none. An item without `url`, or an unknown or ambiguous NAME, is a usage
error (exit 2).

Decided (0.1.0): `cite list` has its own `--format ieee|bibtex|json`,
which is not the global `--format text|json`. Written after `cite list`,
the flag is the verb's; written before the verb, `--format json` is the
global flag and selects the json list. With neither, the list is IEEE
plain text. The reference list covers the citations the scoped design
reaches through the lowered IR (components and their members, and what
they read); citations used only by tables or processes are listed once
those are lowered.

`names` and `bind` are what tools use instead of reimplementing the
mangling rule or the bindings file format (solidworks.md 3). `names`
lists paths sorted by canonical path; the binding state of a path is
`explicit` (bindings file or pin), `implicit` with the reason of ir.md 3
(`global`, `link`, `frame`, `reference`), `unbound`, or `unknown` when
the adapter could not read the CAD files on this machine (an emit-only
adapter, or R008), so the output never guesses. `bind` refuses a locator that fails the
validation of adapters.md 3 and is idempotent: binding an identical
locator again changes nothing.

Decided (0.1.0), frob:ticket 01M4VB644Y6YPK6YTBP4T5YM3Y and
01M4VPEFSVBXSEKWHTQ48SAEKA: `names` asks an adapter that can read (a
`command`, the `read` capability and literal `files`) for its CAD, so a
path without a locator is `unbound` and native-link discoveries
(adapters.md 3.5) show as implicit `global`, `link`, `frame` or
`reference`. An adapter that cannot read, or whose spawn or read fails,
keeps every such path `unknown`. A locator is
identified by kind, locator text, `--for`, `--config` and `--component`;
`bind` of an existing identity with other fields is a conflict (exit 2:
remove it first), and `bind --remove` of an absent locator is a no-op.
`bind` also refuses a locator whose native name another path already
uses (R013 rule, exit 2). The file is rewritten in canonical order
(entries by path, one inline table per locator), which drops comments.
`--tolerance drive|ignore` writes the locator `tolerance` field
(adapters.md 3.6) and is valid on a dimension locator only (exit 2
otherwise). The field is not part of the identity, so binding the same
identity with another tolerance is the conflict above. `names` takes at most one ADAPTER-ID.

### 3.4 `ack` details

Decided (0.1.0), frob:ticket 01M4ST8F1W8T0BKARAMG0D49J2:

- Usage errors (exit 2): no `--role`, a role not declared under
  `[obligations.roles]`, a selector matching no obligation, no `--reason`
  (unless `--baseline` or the selector matches only `waiver:` ids), no
  attester identity (`--as EMAIL`, else `git config user.email`), or a
  role refused by the role policy (obligations.md 5). A refusal writes
  nothing: `ack` is atomic.
- `--baseline` skips obligations the role already attested and records
  reason `baseline` on the rest.
- `--release` restricts the ack to roles that gate `check --release`
  (`required_for` of `check` or `release`); others are skipped.
- An ack at an unchanged digest changes nothing, not even `by` or `at`.
- An unknown author fails closed for `distinct_from_author` roles
  (obligations.md 5).
- `ots:` acks also record the file citations under role `source`.

### 3.5 `obligations` details

Decided (0.1.0). Rows are open and stale obligations only; `--stale`
keeps stale rows. A row's state is judged against the attestation of the
role named by `--role`, else of the first role (by name) whose `covers`
matches; an obligation no role covers is open. `--role R` drops
obligations R does not cover (unknown R is exit 2). Each row prints id,
role, last attester, age in days as of the report date, nearest changed
upstream node and, when the stale verdict still holds, "verdict
unchanged, margin A -> B". Exit 1 when any listed row is required for the
gate (role or kind `required_for`; `--release` adds the `release` ones),
so with `--stale` only stale rows count. `--format json` prints one
`cadetta.obligations/1` document: `as_of`, `as_of_source`, `gated` and an
`obligations` array of the row fields.

### 3.6 Lowered-source verbs

Decided (0.1.0). `why`, `impact`, `ir`, `obligations`, `ack` and `check` all
lower the sources through one analysis and stop with exit 2 when a syntax
error halted it. `ir` prints the lowered design IR with digests. A check
reads the members whose canonical path occurs in its expression, or that
sit in its scope and whose bare name is an identifier token of it. A check
obligation's covered digest is the closure digest of the check node over
those members, so editing the check or anything upstream stales it. `ack`
records the verdict as the check's primary verdict and the margin as `-`
(the IR carries verdicts, not margins), with no citations; only check
obligations are offered so far. `why --stale OBLIGATION [--role R]` judges
the attestation of R (default: the first covering role) and prints
`cadetta.why/1` whose `chain` traces the check and whose `stale` object
carries `state` (`open`, `current`, `stale`) and, when stale, `nearest`
`{node,new,old,depth}`, `others` and `removed`.

### 3.7 `emit` and `import` details

Decided (0.1.0). `emit` words containing `*`, `::` or `.` are path globs and the rest are adapter
ids; `--module` repeats. For each configured adapter it writes the adapter's `out` (default
`build/solidworks/<id>.txt` for `sw-equations`, `build/<id>.json` and `build/<id>.csv` for the built-in `json` and `csv`) under the package directory. Only bound,
non-`let` paths are emitted; a `[*]` binding is emitted once, at instance 0. The `sign = -1`
negation is applied by `emit` to the written text of value and global lines (the adapter library
leaves it to the caller); a `ref`-mode dimension line carries no value, so `sign` does not apply
to it. Q004, R013, R014 and R015 withhold that adapter's file and give exit 1, each reported in the
`--format json` `diagnostics` list as a `cadetta.diag/1` record (`schema`, `severity` `error`, `code`, `message`, `path`); an unknown adapter id, a bad
flag or a halted analysis give exit 2. `json` and `csv` write one row per bound `global` value locator in dependency order (emitters/json.md, emitters/csv.md) and leave an unchanged file alone, comparing modulo the header. External adapters (those with a `command`) are spawned and sent `emit` (adapters.md 5); their files are listed in the report and an adapter failure gives exit 2.

`import ADAPTER FILE...` runs the external adapter's `read` (a `command` containing `/` is
relative to the package directory; no PATH search), builds one observed project per native file
and writes `build/import/proposal.toml` and `proposal.cdt` only. Member names follow adapters.md
7.2 (snake_case, trailing unit stripped, `_2` suffix on collision with the item flagged in its
`reason`). `--assume-unit U` relabels the unit of every unitless value in the proposal. `import
--accept` reads the reviewed proposal and writes `bindings/<id>.toml` (the configured `bindings`
path), `cad/imported/<Component>.cdt` and an `adopted` lock record per file. It never overwrites:
an existing `.cdt` is left alone, and bindings are appended to for paths and locators not yet
bound; the rest are reported as `unchanged`. `skip` items are not adopted. Every import failure
is exit 2, never 1.

`import ADAPTER FILE... --update` re-reads the native files and, for each bound path whose CAD
value differs from the lock's `synced` value, prints a diff of the `.cdt` literal edit (`- 5mm`
/ `+ 12mm`). Only plain literals are edited, never expressions; tolerance/fit literals and
`let`/derived paths are reported as skipped, and a cited literal is flagged. Without `--write`
nothing on disk changes. With `--write` the literals are rewritten and the lock's `synced`
values updated (adapters.md 7.4).

### 3.9 Conformance entries on IR JSON

Decided (0.1.0). Two entries take a `cadetta.ir/1` document instead of a
project; the conformance runner uses them for `kind = "ir"` fixtures
(tests/conformance/README.md). They are not in the table of section 3
because they read no project and need no `cadetta.toml`.

| Entry | Does | Exit |
|---|---|---|
| `ir-validate FILE` | Load FILE and check it against the committed JSON Schema and the invariants of ir.md 6. Prints nothing on success and one line per violation (`<json pointer>: <message>`) on failure. No diagnostic code: an invalid document is not a design. | `0` valid; `2` invalid, unreadable or not JSON |
| `lint FILE [--format json]` | Run the IR-only lint rules (lints.md 4; D001-D003, F004, M, N, Q006, S) on the IR in FILE with catalog severities and no `[lint]` overrides, waivers or project context. A document `ir-validate` rejects exits `2` before any rule runs. `--format json` prints the `check.json` diagnostics (5). | `0` no error; `1` an error-severity diagnostic; `2` as above |

`lint FILE` is how D003 (ir.md 6, acyclic `deps`) is exercised on a
hand-written graph: the IR is the input, so no source can be written that
lowers to a cycle. Rules that need bindings, observed IR or the lock (R and
O families) are not run by `lint FILE`.

Implemented today: `ir-validate` checks the JSON Schema plus the ir.md 6
invariants that read only the document (sorting, path and variant-key
forms, bound shapes, exactness, observed-flavour shape, provenance, tol
terms); `lint FILE` decodes the document (`cadetta_ir::decode`) and runs
D001-D003, F004, M001-M003, N, Q006 and S001/S002/S008. The decoder reads
every top-level entity: dependencies, components (members, parts, fits,
datums, points, placements), checks, solvers, bindings, citations,
interfaces, tables, waivers and the observed frames and instances. D002 sees
the table cells of the document (named `Table[key].column`); S002 (no
package directory) treats every cited file as present. A span's file is
resolved against the directory of FILE and its byte offsets become the real
line and column; when that file is unreadable the location falls back to
`1:1`.

### 3.8 `lsp`

`cadetta lsp` is a Language Server Protocol 3.17 server on stdin and
stdout, for every editor with an LSP client. It is a thin layer over
`check` and `fmt`: it adds no rule and no diagnostic of its own, so the
editor shows exactly what `check` reports.

- Transport and logging: LSP base protocol (Content-Length framing) on
  stdio. stdout carries protocol only; logs go to stderr through
  `cadetta-log`, filtered by `CADETTA_LOG`, and `window/logMessage` is
  never used for diagnostics.
- Capabilities: `textDocumentSync` (open, change with full
  text, save without text, close), `documentFormattingProvider`,
  `hoverProvider`, `codeLensProvider` (no resolve),
  `definitionProvider` and `positionEncoding`. Completion is later (a
  ticket).
- Queries (answered from the package as last checked on disk, so they
  are empty until the first check finishes and lag unsaved edits;
  only members carry a source span, so checks and solvers get none):
  hover over a member declaration shows its `why` chain as Markdown;
  one code lens per member declaration, at its start, titled `impact:
  N dependents` (or `no dependents`) with the client command
  `cadetta.impact` and the canonical path as its only argument; go to
  definition on a full canonical path, or on a name the enclosing
  member reads (matched by last path segment among its dependencies),
  answers the declaration's location, else `null`.
- Projects: each open `.cdt` belongs to the package found by the
  config-cli.md 1.6 walk from its directory; a document outside any
  package gets only syntax diagnostics. Several packages may be open at
  once; each is checked separately.
- On open and on every change the server lexes and parses the buffer
  text and publishes its E001 diagnostics for that document at once.
  On open and on save it runs the full `check` pipeline for the
  document's package against the files on disk (unsaved buffers do not
  take part), and publishes the result for every file of the package,
  replacing what it published before; a file with no diagnostics gets
  an empty list so stale ones clear. A check already running for the
  package is superseded: its result is dropped.
- Mapping from `cadetta.diag/1` (lints.md 1.1): `range` from the
  location's start and end (1-based line and column, columns in Unicode
  characters) to 0-based LSP positions in the negotiated encoding;
  `severity` error 1, warning 2, info 3, hint 4; `code` the diagnostic
  code; `codeDescription.href` absent; `source` `"cadetta"`; `message`
  the message followed by its `note:` and `help:` lines; `data` holds
  the canonical `path` and `variant` when present. A waived diagnostic
  is not published. A diagnostic whose location is in `cadetta.lock`
  is published on that file; one with a native location or none is
  logged, not published.
- Position encoding: `utf-32` when the client offers it (columns are
  characters already), else `utf-16`, converting from the document
  text.
- Formatting returns one whole-document edit with `fmt`'s output, or no
  edit when the text is already canonical; a buffer with syntax errors
  returns an error response (`fmt` refuses it) and leaves the text
  untouched.
- `shutdown` then `exit` ends the process with status 0; `exit` without
  `shutdown` with status 1, as LSP requires.

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
| bindings | `bindings/<id>.toml` (or `[[adapter]] bindings`) | `import --accept`, `bind`, by hand | yes |
| solver results | `<stem>.out.json`, beside the script | `solve` | yes |
| imported scaffold | `cad/imported/<File>.cdt` | `import --accept` | yes |
| pinned citations | `cites/` | `cite pin` | yes (or LFS) |
| JSON schemas | `docs/schemas/` | generated from the Rust types | yes |
| emitted files, proposals | `build/` (`build/import/`, adapter `out` defaults) | `emit`, `sync`, `import` | no (gitignored) |
