# Lint catalog

Every diagnostic has a stable code: one family letter and three digits.
Codes are never reused; retired codes stay listed. Every diagnostic
carries a canonical path and, for design IR, a source span.

## 1. Severity and gating

Severities: `error`, `warning`, `info`. `cadetta check` exits 1 when any
diagnostic is at or above `[lint] fail_on` (`error`, `warning`, `info`
or `never`; default `error`). Severity is configurable per code in
`cadetta.toml [lint]`, except codes marked "fixed" (they report a broken
program, not a style).

Severity precedence, highest first:

1. A fixed code keeps its catalog severity.
2. `required_for` (obligations.md 4) sets a floor for the obligation
   codes O001, O002, O004 and S003: a role or kind required for the
   current gate is at least `error`.
3. `[lint] severity` overrides the catalog default, but for the codes in
   2 it can only raise the floor, never lower it.
4. `waive` and `allow` then suppress the result (section 2).

Engine decisions (0.1.0): `fail_on = never` leaves severities as
computed and only makes the exit code 0. A `[lint] severity` entry for a
fixed code is ignored (logged), an unknown code or severity is a
configuration error. Q004 and R008 default to warning and info under
`check` and to error under `sync`. A waiver is expired when `--as-of` is
strictly after its `expires`, stale when any recorded target digest
differs from the current closure; both stop suppressing. An unscoped
waiver applies under every alternate; `for alt` applies to that variant
only. Fixed codes and diagnostics on a `require` path are never
suppressed. Suppressed diagnostics stay in the run with `waived` set and
do not count toward `fail_on`.

Date-dependent rules (W001, W005, L001) read `--as-of` (obligations.md
5), so a result is reproducible.

### 1.1 Diagnostic record, `cadetta.diag/1`

Every diagnostic, from the compiler, lints, solvers and adapters, is
one record; solver and adapter responses reuse it unchanged. The schema
is `docs/schemas/diag.json`, generated from the Rust type, and the
Python models are generated from it.

```json
{ "schema": "cadetta.diag/1", "code": "C001", "severity": "error",
  "path": "acme.rover::Rover.load_ok", "alt": "wheel=Colson2Series",
  "message": "false for every value in tolerance",
  "location": { "kind": "source", "file": "cad/rover.cdt",
                "start": {"line": 41, "col": 3},
                "end": {"line": 41, "col": 28} },
  "notes": [ { "message": "wheel.mass = 1.2kg", "location": null } ],
  "fixes": [ { "title": "use the instance",
               "edits": [ { "file": "cad/rover.cdt",
                            "range": {"start": {"line": 41, "col": 9},
                                      "end": {"line": 41, "col": 14}},
                            "text": "Rover/wheel" } ] } ],
  "waived": null }
```

| Field | Meaning |
|---|---|
| `code`, `severity` | Catalog code; effective severity after precedence. |
| `path` | Canonical path of the subject, or null. |
| `alt` | `SLOT=ALTERNATE` when raised under variant evaluation, else null. |
| `message` | One sentence, no trailing period. |
| `location` | Where to look; null when there is none. One of `{kind:"source", file, start, end}` (design IR; required for every diagnostic on design IR), `{kind:"native", adapter, file, locator}` (R rules, adapters), `{kind:"lock", line}` (O, W and sync records). |
| `notes` | Related locations and facts: `{message, location}`. |
| `fixes` | Suggested edits (N006, C003, `impact --swap` suggested edit): `{title, edits[{file, range, text}]}`. Never applied without a flag. |
| `waived` | Null, or `{waiver, because}` when suppressed. Suppressed diagnostics appear in JSON only. |

Records are sorted by (file, line, code, path), so output is
deterministic.

### 1.2 Rendering

Text is the default in every context; JSON only by `--format json` or
`CADETTA_FORMAT=json`, never by terminal detection. Text is rustc-style:

```
error[C001]: false for every value in tolerance
  --> cad/rover.cdt:41:3
   |
41 |   check wheel.mass < 1kg
   |   ^^^^^^^^^^^^^^^^^^^^^^ under wheel=Colson2Series
   |
   = note: wheel.mass = 1.2kg
   = help: use the instance
   = path: acme.rover::Rover.load_ok
```

| `--format` | Output |
|---|---|
| `text` | The rendering above (default). |
| `short` | One line, `file:line:col: severity[CODE]: message`; grep and problem-matcher friendly. |
| `github` | GitHub workflow annotations. |
| `sarif` | SARIF 2.1.0 for code scanning. |
| `json` | One `cadetta.diag/1` per diagnostic inside the command's report. |

`cadetta lsp` publishes these records as LSP diagnostics
(config-cli.md 3.8); `cadetta check --format json --watch` emits one
`cadetta.diag/1` per line as files change, for problem matchers and
scripts. Hover as `why` and code lens as `impact` come later.

Short and github rendering rules: `short` is one line per diagnostic;
a diagnostic with no location omits the `file:line:col: ` prefix, and
native and lock locations use the same `file:1:1` and `cadetta.lock:LINE:1`
positions as text. `github` emits `::LEVEL file=F,line=L,col=C,endLine=L,endColumn=C,title=CODE::MESSAGE`
with level `error`, `warning`, or `notice` (info and hint); native and lock
locations give `file` and `line` only, no location gives `title` only.
`%`, CR and LF are escaped in the message, and also `:` and `,` in properties.

SARIF rendering rules: one run, tool `cadetta`, `rules` are the distinct
codes present, sorted; `level` is `error`, `warning`, or `note` (info and
hint). Source locations map to `physicalLocation` regions; native
locations map to `file` at line 1 col 1 with `adapter` and `locator` kept
in the location `properties`; lock locations map to `cadetta.lock`. `path`
is a logical location, `alt` a result property, notes are
`relatedLocations`, fixes map to `fixes`. Waived diagnostics are omitted.

Text rendering rules: the output is ASCII and carries no color codes.
The excerpt shows the first line of the span, underlined to the span
end (to the end of the line for a multi-line span); `alt` is the label
after the carets. When the source line is unavailable, the excerpt is
omitted and `-->` remains. A native location points at `file:1:1`, a
lock location at `cadetta.lock:LINE:1`. A note with a location appends
`(file:line:col)`. Each fix contributes one `= help:` line from its
title. Order is `note`, `help`, `path`.

## 2. Suppression

### 2.1 Waivers

```
waive D001 on acme.parts::Bearing608.* for alt wheel=Colson2Series {
  because "vendor dimensions coincide with ours by design"
  expires 2027-01-01
}
```

Grammar: `waive CODE on GLOB [for alt SLOT=ALT] { because STRING
[expires DATE] }` (production in language.md 3). GLOB is the path glob
of obligations.md 2.2, shared with obligation selectors. A waiver
suppresses one code on the paths matched. `for alt SLOT=ALT` scopes it
to one alternate's variant in one slot (language.md 3.2).

Digests: when reviewed (`cadetta ack waiver:GLOB`), the closure digests
of the waiver's targets are stored in the lock as a `waiver` record
(obligations.md 3). A waiver with no record yet is open: it suppresses,
and its review is an open `waiver` obligation (O002) until `ack`. Once
recorded, if a target's digest differs, the waiver is stale: W002 fires
and the suppressed diagnostic reappears until re-reviewed. `cadetta check`
compares the recorded digest with the combined closure digest of the
waiver's current targets, and reports W005 for waivers expiring within
`expiry_warn_days` of `--as-of` (`eol` dates are not yet carried by the IR).

| Code | Condition |
|---|---|
| W001 | The waiver's `expires` date has passed (`--as-of`); it stops suppressing. |
| W002 | A target's closure digest differs from the lock record: re-review. The suppressed diagnostic reappears. A waiver with no record is open (O002), not stale. |
| W003 | The waiver names a fixed code, or a diagnostic raised by a `require`. |
| W004 | The waiver (or `allow`) matched no diagnostic: it suppresses nothing. |
| W005 | A waiver or an `eol` date falls within `[lint] expiry_warn_days` (default 30) of `--as-of`. |

### 2.2 `allow`

`#[allow(CODE, "because")]` is a one-member waiver written on the
member: `#[allow(M001, "vendor drawing 1234")]`. The reason string is
required. It has no expiry and is excluded from W001, W002 and W005,
and it needs no lock record; W004 does check it. Fixed codes cannot be
allowed (W003).

### 2.3 `require`

The single `require` rule is stated in obligations.md 4 and applies
here without exception: a `require` is never suppressed; an attestation
is sign-off, never a waiver. C001 on a `require` stays an error, and
naming it in `waive` or `allow` is W003.

## 3. Catalog

### E: resolution and evaluation

All E codes are fixed errors unless noted: they report a program that
cannot be resolved or evaluated.

| Code | Name | Default | Fixed | Meaning |
|---|---|---|---|---|
| E001 | syntax-error | error | yes | The source does not lex or parse (language.md 2, 3); lexical errors (quantities.md 2: leading or trailing `.`, `5"abc"`, bad compound literals) are E001 with a lexical message. |
| E002 | unresolved-name | error | yes | A name, path or `use` target resolves to nothing. |
| E003 | duplicate-definition | error | yes | Two definitions have one canonical path; both spans are given. |
| E004 | ambiguous-import | error | yes | Two globs or uses bring the same name, where the name is used. |
| E005 | missing-package | error | yes | A package's `src` is missing or holds no `.cdt` file, or a workspace member or path dependency has no `cadetta.toml` (packages.md 1). |
| E006 | argument-mismatch | error | yes | Arguments to a component, `part`, function or fit do not match the parameters (unknown, missing, wrong kind); also a `cite` with an unknown type or field, a malformed date or URL, or a value of the wrong shape (language.md 3.9), and a `position` tolerance where it is not allowed (geometry.md 3.1). |
| E007 | lookup-miss | error | yes | A table lookup key matched no row. |
| E008 | table-shape-mismatch | error | yes | A table row or column does not match the table's `key` and `cols`. |
| E009 | non-exhaustive-match | error | yes | A `match` leaves some case unhandled. |
| E010 | role-required | error | yes | `mmc` or `lmc` read on a tolerance with no role (hole or shaft) to say which side is material. |
| E011 | parameter-dependent-type-read | error | yes | `Type.member` read where the member depends on component parameters; a parameterized component exists only as instances (language.md 3.1). |
| E012 | version-unsatisfied | error | yes | A resolved dependency does not meet the `version` requirement (packages.md 1). |
| E013 | version-conflict | error | yes | Two requirements on one package cannot be met by a single version. |
| E014 | dependency-changed | error | yes | A dependency's content differs from the digest pinned in the lock; run `cadetta update`. |
| E015 | bad-module-name | error | yes | A file or directory under `src` is not a valid module name. |
| E016 | private-item | error | yes | An item is used outside its visibility, or a `priv` member is read from another package (packages.md 3). |
| E017 | private-in-public | error | yes | A `pub` item's signature names a less visible item, or a member implementing an interface member is `priv`. |

E003 and E004 attach the second definition or import span as a note on the diagnostic. Every E code is a fixed error: configuration cannot change its severity.

### Q: quantities

| Code | Name | Default | Fixed | Meaning |
|---|---|---|---|---|
| Q001 | unitless-quantity | error | yes | Bare number where a quantity is required (quantities.md 2). |
| Q002 | dimension-mismatch | error | yes | `+ -` or comparison across dimensions. |
| Q003 | kind-mismatch | error | yes | Implicit conversion between named kinds. |
| Q004 | inexact-emit | error (emit), warning (check) | | Value not representable at the target's declared precision. At emit time it is an error: the value is withheld, that path is not written and `sync` exits 1. `check` has no target to write, so it reports a warning. |
| Q005 | (retired) | | | Was unsupported-toleranced-arithmetic; now conservative evaluation. |
| Q006 | inexact-value | info | | Reported once at the root inexact operation, with the count of dependents. |
| Q007 | quantity-literal-split | error | yes | Whitespace between number and unit. |
| Q008 | angle-dropped | warning | | Angle exponent mismatch (quantities.md 4.3): the missing-2*pi class. |
| Q009 | non-integer-count | error | yes | A Count that is not an integer. |
| Q010 | ambiguous-rounding | error | yes | Rounding an inexact value that sits on a tie or step boundary. |
| Q011 | root-of-dimension | error | yes | `sqrt` or `cbrt` of a dimension whose exponents do not divide. |

### N: naming

| Code | Name | Default | Fixed | Meaning |
|---|---|---|---|---|
| N001 | non-canonical-case | warning | | Name breaks the case canon: package `lower.dotted`, module `snake_case`, type `PascalCase`, rest `snake_case` (language.md 4). |
| N002 | redundant-prefix | warning | | Member repeats its component's name (`Axle.axle_length`). |
| N003 | unit-in-name | warning | | Name encodes a unit (`diameter_mm`). |
| N004 | renamed-reference | warning | | Reference uses a `was` path, including one naming another module, file or package (packages.md 5). |
| N005 | reserved-separator | error | yes | Name contains `__` (reserved for mangling). |
| N006 | type-read-beside-instance | warning | | `Type.member` in an assembly that has a `part` of that type; suggests the instance (language.md 3.1). |

N002 matches the component name in `snake_case` or flattened lowercase
followed by `_` (`Axle.axle_length`, `FrontAxle.front_axle_x`). N003 fires
when the last `_` token of a member name is a unit symbol (`mm`, `in`,
`kg`, `deg`, `rpm`, `mpa`, and similar). A name containing `__` raises
N005 only, never also N001. N004 findings come from the resolver, since
only it sees the `was` text; the lint turns them into diagnostics.
N006 carries its suggestion in the message (`rear.length`); no edit is
attached because the read has no single source range in the IR.

### D: duplication and single source of truth

| Code | Name | Default | Fixed | Meaning |
|---|---|---|---|---|
| D001 | restated-value | warning | | An authored literal in a `model` (or observed IR) equals another parameter's value of the same kind, in any component including its own. Scored by specificity: fires at the configured severity when the literal has at least `[lint] d001_min_digits` significant digits (default 3: `12.012mm`, `6.35mm`), at info below that, and never for `[lint] common_values` unless either side is imported (observed IR), where a coincidence of round numbers is still worth a look. Significant digits are counted in the authored form from the first to the last non-zero digit (`100mm` has 1, `6.35mm` 3). One diagnostic per group of equal values, listing every location. Never between two `ots` parameters. |
| D002 | derivable-authored | warning | | A literal equals a value the design already offers by name: a fit member or its accessors, a table cell reachable from the component, or a solver output. `12.009mm` equals `seat.shaft.mid`. Not in an `ots` for a cited literal: a vendor's stated value is its own source even when it matches a standard (T001 covers mismatches). |
| D003 | reference-cycle | error | yes | Parameters depend on each other cyclically. |

Implementation rules (decided with the D rules): a D001 group takes the largest
significant-digit count among its members to choose between the configured severity and info; a
group needs at least one literal in a non-`ots` component, so derived values and `ots`-only groups
never fire; D002 compares against `nominal`, `min`, `max` and `mid` of fit members and the fit
clearance, solver output members, and table cells the caller passes in (the IR carries no cells);
D003 is computed on the member dependency graph, so a loop between components with no member-level
loop is clean.

### M: magic numbers

| Code | Name | Default | Fixed | Meaning |
|---|---|---|---|---|
| M001 | unexplained-precision | warning | | In a `model`: a literal with more significant decimals than `[lint] precision` for its kind and no tolerance, fit, table lookup or allow. Provenance and doc comments do not explain a value. In an `ots`: the same, unless cited. |
| M002 | magic-factor | info | | Bare factor other than small integers, simple fractions and powers of ten. |
| M003 | untoleranced-fit-side | warning | | A parameter used as a side of a `fit` has no tolerance. |

Implementation rules (decided with the M rules): the M rules look at authored parameter literals
only (no expression, nothing read), so a table lookup never reaches them and `allow` is applied by
the engine. M001 counts decimals in the authored number, ignoring trailing zeros (`12.50mm` has 1),
against `[lint] precision` for the value's kind; a kind with no entry is never reported. A
tolerance or use as a fit side explains a literal; a citation explains it only in an `ots`. M002
applies to dimensionless literals that are uncited: plain means an integer of magnitude at most 10,
a fraction with numerator and denominator at most 10, or a power of ten (`1000`, `0.01`). M003 is
reported once per fit-side member without a tolerance.

`cadetta check` threads `[lint] precision`, `d001_min_digits` and `common_values` into M001, D001
and D002. An empty `precision` keeps the built-in defaults (Length 2, Angle 1); a `precision` key
that is not a built-in quantity kind, or a `common_values` entry that is not a quantity, is
rejected when the configuration loads. D002 also sees the quantity cells of every `table` in the
package and its dependencies, named `Table[key].column`. A D001 below `d001_min_digits` stays at
info even though the engine otherwise resets severities, unless `[lint] severity` sets `D001`.

### F: fits

| Code | Name | Default | Fixed | Meaning |
|---|---|---|---|---|
| F001 | unknown-fit-class | error | yes | Class not in the ISO 286 tables. |
| F002 | size-out-of-range | error | yes | Nominal outside the table's size range. |
| F003 | fit-tolerance-conflict | error | | A side's own tolerance numerically differs from the class it is declared to play. |
| F004 | fit-range-violated | error | | A fit's clearance lies entirely outside its declared required range; a partial overlap is C002. |

### I, L, T, P: interfaces, lifecycle, tables, processes

| Code | Name | Default | Fixed | Meaning |
|---|---|---|---|---|
| I001 | missing-interface-member | error | yes | An implementer lacks an interface member. |
| I002 | alternate-not-viable | warning | | An approved alternate violates a check, fit, bound or require under variant evaluation (names it). |
| I003 | alternate-undecided | info | | An approved alternate is viable but leaves a check undecided that the primary decides. |
| I004 | root-not-evaluated | info | | A root of the package being checked has parameters without defaults and no instances, so its members are never evaluated. Private unused parameterized components are U002. |
| L001 | end-of-life-part | warning | | A primary is `eol` or `obsolete` and has no viable alternate. Error once its date has passed. |
| L002 | not-recommended-part | info | | A primary is `nrnd`. |
| T001 | not-in-table | warning | | An `ots` literal citing a table matches none of its rows. |
| P001 | missing-process-role | warning | | Under a process, a feature-like parameter has no rule role. |

I rule implementation notes (0.1.0): I001 checks every non-optional
interface member and every interface datum by member-root name. I002
and I003 read the per-alternate check verdicts only (fits, bounds and
requires reach them through checks); I002 is raised per check and
alternate, and only when the primary does not itself fail that check
(then it is C001). I003 requires the alternate to fail no check. I004
skips private components (U002) and components some part names as its
type, primary or alternate.

Engine entry points (0.1.0): `cadetta check` runs I001 (`check_members`,
interfaces lowered from the AST with `extends` flattened; bounds are not
yet lowered), I004 (`check_roots`) and U002 (`u002::check`). U002 decides
"used" lexically: an item is used when its identifier is mentioned more
than once across the package's modules (its own declaration counts
once). `require` protection is wired by collecting the path of every
obligation check into the suppression table, and `#[allow(CODE, reason)]`
member attributes are collected from the IR into `allows`.

L, T and P rule implementation notes (0.1.0): the facts are decided
upstream (lifecycle analysis in cadetta-variants, the table evaluator and
the process pass in cadetta-sema) and handed to `cadetta-lint`
`rules::lifecycle`, which lifts them to diagnostics. L001 is a warning
before its date and an error once the date has passed `--as-of`
whatever the configured severity; L002 lists viable alternates in its
message. T001 and P001 use the configured severity.

Wiring in `cadetta check` (0.1.0): `pipeline::analyze_at` feeds the lifts.
Q007 comes from the lexer: it is lifted as a fixable error carrying the join
fix, while E001 stays a halting syntax error. P001 comes from the process
pass (one fact per feature-like parameter with no role). F001/F002 (fit
class or size out of the ISO 286 tables) and F003 (side tolerance disagrees
with the class) come from fit derivation. T001 compares each literal of a
non-parameterized `ots` component (the member citation, else the component
citation, when it names a `cite` path) with every key and cell of the
tables carrying the same citation path; it fires only when a table holds
values of the literal's dimension and none equals it. L001/L002 read the
`lifecycle` attribute of the primary of each part slot. `--as-of` (else
`[package] as_of`, else today) is the date for L001. An
alternate is viable when it names a known `ots` component that is not
itself end-of-life and the design evaluated with that alternate swapped
into its slot violates no check or require (language.md 3.2); undecided
results keep it viable. Swaps are evaluated only for non-`active` primaries.
E-family
resolution errors keep their current fixed-error text and are not re-lifted
through `lift_resolution`.

### S: sources and solvers

| Code | Name | Default | Fixed | Meaning |
|---|---|---|---|---|
| S001 | ots-without-citation | warning | | `ots` value with no citation on it or its component. |
| S002 | citation-unreachable | info | | A citation's `file` does not exist. URLs are never fetched by `check` (`cadetta cite pin` makes a URL checkable). |
| S003 | solver-stale | warning | | A solver result was computed against inputs that have changed. |
| S004 | citation-changed | warning | | A citation's digest (its fields and, with `file`, the file's digest) differs from the acknowledged one. The only diagnostic for citation staleness; O001 is not raised for the same node. Only a `file` is drift-detected on disk; `cadetta cite pin` vendors a URL. |
| S005 | solver-failed | error | | The last solve failed; the previous result is in use. |
| S006 | solver-unsolved-variant | warning | | A variant changes a solver's inputs and no result exists for them; dependent verdicts are undecided (solvers.md 4.1). |
| S007 | solver-input-not-quantity | error | yes | A solver `in` expression is not a quantity-valued expression, or an `out` has no kind (solvers.md 1). |
| S008 | citation-incomplete | warning | | A `cite` lacks a field its type needs to find the source again, or has a `url` with neither `file` nor `accessed`; or an `ots` value's only citation is an inline `@"note"` (language.md 3.9). Pseudo-citations are never S008. |

### C: checks

| Code | Name | Default | Fixed | Meaning |
|---|---|---|---|---|
| C001 | check-violated | error | | False for every value in tolerance (names the alternate when relevant). |
| C002 | check-undecided | warning | | Holds for some values and not others; says if the evaluation was conservative. |
| C003 | toleranced-equality | warning | | `==` on toleranced operands; suggests accessors. |
| C004 | derived-feature-in-check | info | | A check reads an arithmetic parameter with toleranced inputs and no tolerance of its own, so it treats a computed value as exact (quantities.md 5.3); suggests `let`. |

C rule implementation notes (0.1.0): `cadetta-lint` `rules/c.rs` reads
the check verdicts. C001 and C002 are raised per verdict: once for the
primary and once for each alternate that fails or is undecided, the
latter with `alt` set and `for alt SLOT=ALT` in the message, so a waiver
may be scoped to one alternate (language.md 3.2); I002 additionally
warns when only an alternate fails. C001 and C002 are also raised
for each interface bound (`load_rating >= 400N`) an implementing
component's member tolerance range lies wholly outside (C001, error) or
straddles (C002, warning), and C002 for a `fit` required range the
resulting clearance only partly meets. L001 keeps its error severity
once the EOL date has passed, whatever severity is configured. C003 inspects the operands of each
`==`/`!=` in the check text and fires when one reads a toleranced member
of the check's component other than through `.nominal`. C004 fires per
derived parameter (a `param` with an expression and dependencies, no
tolerance of its own) that the check reads while one of its dependencies
is toleranced. The diagnostic sits on the derived parameter (the member to
turn into a `let`), once per parameter however many checks read it.

### O: obligations

O001 and O002 cover role attestations only; staleness of a citation is
S004 and of a solver S003 (no double firing; obligations.md 2.3).

| Code | Name | Default | Fixed | Meaning |
|---|---|---|---|---|
| O001 | obligation-stale | warning | | An attestation's digest set changed (error if its role is `required_for = "check"`). A citing node's digest never includes the cited source's digest (ir.md 4), so a changed citation is S004 alone. |
| O002 | obligation-open | warning | | A required attestation was never given. |
| O003 | requirement-unattested | error | | A `require` that holds lacks the sign-off its role requires for release. |
| O004 | sync-stale | info | | The "CAD file reflects design" obligation is stale. Info by default because CI cannot read CAD; gating by `[obligations.kinds.sync] required_for`. |

### G: positions and datums

Defined with their semantics in geometry.md 7.

| Code | Name | Default | Fixed | Meaning |
|---|---|---|---|---|
| G001 | datum-mismatch | error | yes | Arithmetic or comparison between positions in different datums without `in`; also array + point, and a point offset whose point is not on the datum it offsets. |
| G002 | placement-cycle | error | yes | A placement chain returns to a part already placed; the closing connection is a check. |
| G003 | unplaced-part | error | yes | A position is converted out of a part that has no placement, or re-typed where the model lacks the part or its placement. |
| G004 | pattern-count-mismatch | error | | `aligned` or a joint's `at`/`onto` pair patterns of different counts; an array placement's operands do not have the part array's count, or an indexed placement's anchor is array-valued (geometry.md 4). |
| G005 | bolted-float-unlocated | warning | | A check tighter than a joint's float reads through a placement carrying that float (geometry.md 5.2). |
| G006 | double-placement | error | yes | A part or instance is placed more than once. |
| G007 | unplaced-instance | warning | | A part array placed instance by instance leaves an instance without a placement. |
| G008 | rotation-unconstrained | warning | | A non-locating joint with a one-point pattern carries float onto a placement edge, so the part may turn about the screw (geometry.md 5.2). |

### R: read-back and drift

R rules run on observed IR after `read` or `sync`; R004 and R008-R010
come from the adapter's responses and are passed through unchanged
(adapters.md). The implementation is `crates/cadetta-lint/src/rules/r_adapter.rs`:
R007, R011, R012 and R013 are pure functions over binding facts, and
`pass_through` forwards the adapter codes unchanged.

`cadetta check` runs the family for every `[[adapter]]` with a `command`
that advertises `read` (hook: `cadetta-cli` `check_drift::process_drift`):
the adapter is spawned, its files are read, and R001, R002, R003, R005 and
the adapter's own R004/R009/R010 are reported; an adapter that cannot
start is R008 at info. Nothing is pushed or written. R007, R011 and R012
need observed facts (sketch planes, assembly slots, instance values) that
`read` does not yet carry, so they are not run by `check`.

| Code | Name | Default | Fixed | Meaning |
|---|---|---|---|---|
| R001 | drift | error | | Observed value differs from the value last emitted for that path (the emit point, not the nominal; the base is the lock `synced` record). Says which side moved: cad, design or both. `sync --force-design` pushes over it. Also a dimension tolerance that differs (adapters.md 3.6), and a frame or component transform that differs from the composed datum or placement (adapters.md 3.4); a placement is verify-only, so it has no base. |
| R002 | unbound-native | warning | | Native parameter has no binding. (A `csv` file reports an unbound row as R016 instead.) |
| R003 | missing-native | error | | Bound path not found in the native file, including a native link (adapters.md 3.5) that a sync recorded and that is gone. |
| R004 | regen-failed | error | | The CAD tool failed to rebuild after a push; affected documents are restored from their snapshot. |
| R005 | orphan-binding | warning | | A binding's path matches no IR path and no `was` path. |
| R006 | native-structure-replaced | warning | | A push replaces a native expression or driving equation on a bound locator (cadetta owns the dependency structure); lists the old expression. For an equation-driven dimension the equation is removed, then the value set, reported and shown first by `--dry-run`. Also fires on the first sync after import when the emitted value differs from the legacy value (the `adopted` record). |
| R007 | binding-datum-mismatch | warning | | A bound coordinate's native sketch is not on its datum's plane, or its dimension is not measured from the sketch origin (adapters that can see it; geometry.md 6). |
| R008 | adapter-unavailable | error (sync), info (check) | | Adapter not installed, wrong platform, or version mismatch (exit 2 on sync). |
| R009 | file-busy | error | | Document dirty, locked or read-only; refused before any push. |
| R010 | push-rejected | error | | Push to a driven or suppressed dimension, an equation conflict or a unit conflict. |
| R011 | component-mismatch | warning | | The file instantiated in an assembly is not the file bound to the slot's primary (for example after a manual swap). |
| R012 | binding-divergent | error | | A `[*]` binding over instances whose emitted values differ. |
| R013 | name-collision | error | yes | Two in-scope paths would create the same native name, bind one explicitly (adapters.md 3.1); or two component instances name the same part in their reference property (adapters.md 3.5). |
| R014 | text-not-expressible | error | yes | An emission is required for a bound `Text` parameter and the adapter's format cannot express `Text` (`sw-equations`, emitters/sw-equations.md 1). A `Text` parameter that is not emitted (unbound, filtered out, or a `let`) is never reported. |
| R015 | locator-unsupported | error | yes | An emission is required for a `dimension`, `property` or `frame` locator, or a `mode = "ref"` binding, on an adapter whose format has only `global` value rows (`json`, `csv`; emitters/json.md 1, emitters/csv.md 1). A locator that is not emitted (filtered out, narrowed away, or on a `let`) is never reported. |
| R016 | unbound-row | error | yes | A `read` of a `csv` file meets a row whose `name` matches no binding of that adapter (emitters/csv.md 3). The file is user-supplied and carries no path, so an unbound row is an error rather than the native-discovery warning R002; no observed IR is produced for the file. (`json` entries carry their own `path`.) |

### U, W

| Code | Name | Default | Fixed | Meaning |
|---|---|---|---|---|
| U001 | unused-parameter | info | | A `model` parameter is read by nothing and bound to nothing. |
| U002 | unused-private-item | warning | | A private item is used nowhere in its package. |
| W001 | waiver-expired | error | yes | A waiver's `expires` date has passed; it no longer suppresses (2.1). |
| W002 | waiver-stale | warning | yes | A target's digest changed since the waiver was reviewed; the suppressed diagnostic reappears. A waiver never reviewed is O002, not W002. |
| W003 | waiver-not-allowed | error | yes | Waiver or `allow` of a fixed code or of a diagnostic raised by a `require`. |
| W004 | waiver-unused | warning | yes | A waiver or `allow` suppresses nothing. |
| W005 | waiver-expiring | info | | A waiver or an `eol` date falls within `[lint] expiry_warn_days` (default 30). |

## 4. Rule implementation contract

Each rule is a pure function from IR (plus, for R and O rules, bindings,
observed IR and the lock file) to diagnostics, with positive and
negative fixtures (a must-fire and a must-not-fire) under
`tests/conformance/lints/<code>/`, part of the conformance corpus
(`tests/conformance/<doc>/` for every other spec document). A rule is documented here
before it ships; its code links back with `frob:doc`.

A *literal* for D001, D002 and M001-M003 is an authored parameter that reads
nothing and whose expression text is absent or equal to its authored token
(lowering records a literal's source text as `expr`). `cadetta check` runs the
IR-only families (D001, D002, F004, M, N, Q006, S) with the catalog
severities, then applies `[lint]` overrides, `waive` declarations and the W
rules through the engine.
