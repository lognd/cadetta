# Lint catalog

Every diagnostic has a stable code: one family letter and three digits.
Codes are never reused; retired codes stay listed. Every diagnostic
carries a canonical path and, for design IR, a source span.

## 1. Severity and gating

Severities: `error`, `warning`, `info`. `cadetta check` exits 1 when any
diagnostic is at or above `[lint] fail_on` (default `error`). Severity is
configurable per code in `cadetta.toml [lint]`, except codes marked
"fixed" (they report a broken program, not a style).

## 2. Waivers

```
waive D001 on acme.parts::Bearing608.* {
  because "vendor dimensions coincide with ours by design"
  expires 2027-01-01
}
```

A waiver suppresses one code on the paths matched by a path glob. It
binds the closure digests of its targets at waive time. `for alt NAME`
scopes it to one alternate's variant (language.md 3.2). Fixed codes cannot be
waived (W003). W001 expired, W002 a target changed (re-review), W004
suppresses nothing. A `require` is never waived; it is attested
(obligations.md 4). `#[allow(CODE)]` is a waiver without expiry for one
member and still needs a `because` string: `#[allow(M001, "vendor
drawing 1234")]`.

## 3. Catalog

### Q: quantities

| Code | Name | Default | Fixed | Meaning |
|---|---|---|---|---|
| Q001 | unitless-quantity | error | yes | Bare number where a quantity is required (quantities.md 2). |
| Q002 | dimension-mismatch | error | yes | `+ -` or comparison across dimensions. |
| Q003 | kind-mismatch | error | yes | Implicit conversion between named kinds. |
| Q004 | inexact-emit | warning | | Value not representable at the target's declared precision. |
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
| N001 | non-canonical-case | warning | | Name breaks the case canon (language.md 4). |
| N002 | redundant-prefix | warning | | Member repeats its component's name (`Axle.axle_length`). |
| N003 | unit-in-name | warning | | Name encodes a unit (`diameter_mm`). |
| N004 | renamed-reference | warning | | Reference uses a `was` path. |
| N005 | reserved-separator | error | yes | Name contains `__` (reserved for mangling). |
| N006 | type-read-beside-instance | warning | | `Type.member` in an assembly that has a `part` of that type; suggests the instance (language.md 3.1). |

### D: duplication and single source of truth

| Code | Name | Default | Fixed | Meaning |
|---|---|---|---|---|
| D001 | restated-value | warning | | An authored literal in a `model` (or observed IR) equals another parameter's value of the same kind, in any component including its own. Scored by specificity: fires at the configured severity when the literal has at least `[lint] d001_min_digits` significant digits (default 3: `12.012mm`, `6.35mm`), at info below that, and never for `[lint] common_values` unless either side is imported (observed IR), where a coincidence of round numbers is still worth a look. Significant digits are counted in the authored form from the first to the last non-zero digit (`100mm` has 1, `6.35mm` 3). One diagnostic per group of equal values, listing every location. Never between two `ots` parameters. |
| D002 | derivable-authored | warning | | A literal equals a value the design already offers by name: a fit member or its accessors, a table cell reachable from the component, or a solver output. `12.009mm` equals `seat.shaft.mid`. Not in an `ots` for a cited literal: a vendor's stated value is its own source even when it matches a standard (T001 covers mismatches). |
| D003 | reference-cycle | error | yes | Parameters depend on each other cyclically. |

### M: magic numbers

| Code | Name | Default | Fixed | Meaning |
|---|---|---|---|---|
| M001 | unexplained-precision | warning | | In a `model`: a literal with more significant decimals than `[lint] precision` for its kind and no tolerance, fit, table lookup or allow. Provenance and doc comments do not explain a value. In an `ots`: the same, unless cited. |
| M002 | magic-factor | info | | Bare factor other than small integers, simple fractions and powers of ten. |
| M003 | untoleranced-fit-side | warning | | A parameter used as a side of a `fit` has no tolerance. |

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
| I004 | root-not-evaluated | info | | A root component has parameters without defaults and no instances, so its members are never evaluated. |
| L001 | end-of-life-part | warning | | A primary is `eol` or `obsolete` and has no viable alternate. Error once its date has passed. |
| L002 | not-recommended-part | info | | A primary is `nrnd`. |
| T001 | not-in-table | warning | | An `ots` literal citing a table matches none of its rows. |
| P001 | missing-process-role | warning | | Under a process, a feature-like parameter has no rule role. |

### S: sources and solvers

| Code | Name | Default | Fixed | Meaning |
|---|---|---|---|---|
| S001 | ots-without-citation | warning | | `ots` value with no citation on it or its component. |
| S002 | citation-unreachable | info | | A local-file citation path does not exist. Literal citations (`@"ASTM A276"`) are never checked. |
| S003 | solver-stale | warning | | A solver result was computed against inputs that have changed. |
| S004 | citation-changed | warning | | A cited local file's digest differs from the acknowledged one. |
| S005 | solver-failed | error | | The last solve failed; the previous result is in use. |
| S006 | solver-unsolved-variant | warning | | A variant changes a solver's inputs and no result exists for them; dependent verdicts are undecided (solvers.md 4.1). |

### C: checks

| Code | Name | Default | Fixed | Meaning |
|---|---|---|---|---|
| C001 | check-violated | error | | False for every value in tolerance (names the alternate when relevant). |
| C002 | check-undecided | warning | | Holds for some values and not others; says if the evaluation was conservative. |
| C003 | toleranced-equality | warning | | `==` on toleranced operands; suggests accessors. |
| C004 | derived-feature-in-check | info | | A check reads an arithmetic parameter with toleranced inputs and no tolerance of its own, so it treats a computed value as exact (quantities.md 5.3); suggests `let`. |

### O: obligations

| Code | Name | Default | Fixed | Meaning |
|---|---|---|---|---|
| O001 | obligation-stale | warning | | An attestation's digest set changed (error if its role is `required_for = "check"`). |
| O002 | obligation-open | warning | | A required attestation was never given. |
| O003 | requirement-unattested | error | | A `require` lacks a required attestation for release. |

### G: positions and datums

Defined with their semantics in geometry.md 7.

| Code | Name | Default | Fixed | Meaning |
|---|---|---|---|---|
| G001 | datum-mismatch | error | yes | Arithmetic or comparison between positions in different datums without `in`. |
| G002 | placement-cycle | error | yes | A placement chain returns to a part already placed; the closing connection is a check. |
| G003 | unplaced-part | error | | A position is converted out of a part that has no placement. |
| G004 | pattern-count-mismatch | error | | `aligned` or a joint's `at`/`onto` pair patterns of different counts. |

### R: read-back and drift

R rules run on observed IR after `read` or `sync`; R004 is reported by
the adapter's `regen` result and passed through unchanged.

| Code | Name | Default | Fixed | Meaning |
|---|---|---|---|---|
| R001 | drift | error | | Observed value differs from the value last emitted for that path (the emit point, not the nominal). Says which side moved. |
| R002 | unbound-native | warning | | Native parameter has no binding. |
| R003 | missing-native | error | | Bound path not found in the native file. |
| R004 | regen-failed | error | | The CAD tool failed to rebuild after a push. |
| R005 | orphan-binding | warning | | A binding's path matches no IR path and no `was` path. |
| R006 | adopt-value-change | info | | On first sync after import, the emitted value differs from the legacy native value. |
| R007 | binding-datum-mismatch | warning | | A bound coordinate's native sketch is not on its datum's plane (adapters that can see it; geometry.md 6). |

### U, W

| Code | Name | Default | Fixed | Meaning |
|---|---|---|---|---|
| U001 | unused-parameter | info | | A `model` parameter is read by nothing and bound to nothing. |
| W001 | waiver-expired | error | yes | |
| W002 | waiver-stale | warning | yes | A target's digest changed since the waiver. |
| W003 | waiver-not-allowed | error | yes | Waiver of a fixed code or a `require`. |
| W004 | waiver-unused | warning | yes | |

## 4. Rule implementation contract

Each rule is a pure function from IR (plus, for R and O rules, bindings,
observed IR and the lock file) to diagnostics, with positive and
negative fixtures (a must-fire and a must-not-fire) under
`tests/conformance/lints/<code>/`, part of the conformance corpus
(`tests/conformance/<doc>/` for every other spec document). A rule is documented here
before it ships; its code links back with `frob:doc`.
