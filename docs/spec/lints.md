# Lint catalog

Every diagnostic has a stable code: one family letter and three digits.
Codes are never reused; a retired code stays in this table marked
retired. Every diagnostic carries a canonical path and, for design IR, a
source span.

## 1. Severity and gating

Severities are `error`, `warning`, `info`. `cadetta check` exits 1 when
any diagnostic is at or above `[lint] fail_on` (default `error`).
Severity can be raised or lowered per code in `cadetta.toml [lint]`,
except codes marked "fixed" (they report a broken program, not a style).

## 2. Waivers

```
waive M001 on acme.rover::Axle.chamfer {
  because "matches vendor drawing 1234 rev B"
  expires 2027-01-01
}
```

A waiver suppresses one code on one canonical path. It binds the
target's digest at waive time (ir.md 4). Fixed codes cannot be waived
(W003). An expired waiver is W001; a waiver whose target changed is W002
(re-review it); a waiver that suppresses nothing is W004.

## 3. Catalog

### Q: quantities

| Code | Name | Default | Fixed | Meaning |
|---|---|---|---|---|
| Q001 | unitless-quantity | error | yes | Bare number where a quantity is required. |
| Q002 | dimension-mismatch | error | yes | `+ - compare` across dimensions. |
| Q003 | kind-mismatch | error | yes | Implicit conversion between named kinds. |
| Q004 | inexact-emit | warning | | Value not representable at the target's precision. |
| Q005 | unsupported-toleranced-arithmetic | error | yes | `*` or `/` of two toleranced quantities. |
| Q006 | inexact-value | info | | Value computed through an inexact operation. |
| Q007 | quantity-literal-split | error | yes | Whitespace between number and unit. |

### N: naming

| Code | Name | Default | Fixed | Meaning |
|---|---|---|---|---|
| N001 | non-canonical-case | warning | | Name breaks the case canon (language.md 4). |
| N002 | redundant-prefix | warning | | Parameter repeats its component's name (`Axle.axle_length`); the path already qualifies it. |
| N003 | unit-in-name | warning | | Name encodes a unit (`diameter_mm`); quantities carry units. |
| N004 | renamed-reference | warning | | Reference uses a `was` name. |

### D: duplication and single source of truth

| Code | Name | Default | Fixed | Meaning |
|---|---|---|---|---|
| D001 | restated-value | warning | | An authored literal equals another parameter's value of the same kind in a different component; probably a copy that should be a reference. Info-level when the value is a common round number (configurable list). |
| D002 | derivable-authored | warning | | An authored literal equals an expression the compiler can form from dependencies already present (e.g. exactly twice an existing value, or a fit member). |
| D003 | reference-cycle | error | yes | Parameters depend on each other cyclically. |

### M: magic numbers

| Code | Name | Default | Fixed | Meaning |
|---|---|---|---|---|
| M001 | unexplained-precision | warning | | Literal has more significant decimals than `[lint] precision` for its kind and has no tolerance, fit, source or doc comment. `12.012mm` is the canonical example. |
| M002 | magic-factor | info | | Bare numeric factor in an expression other than small integers and simple fractions. |
| M003 | untoleranced-interface | warning | | Parameter referenced by a `fit` or `check` against another component has no tolerance. |

### F: fits

| Code | Name | Default | Fixed | Meaning |
|---|---|---|---|---|
| F001 | unknown-fit-class | error | yes | Class not in the ISO 286 tables. |
| F002 | size-out-of-range | error | yes | Nominal outside the table's size range. |
| F003 | fit-tolerance-conflict | error | | The fitted parameter's own tolerance disagrees with the fit side it plays. |

### S: sources and obligations

| Code | Name | Default | Fixed | Meaning |
|---|---|---|---|---|
| S001 | ots-without-source | warning | | `ots` value with no `source` on it or its component. |
| S002 | source-unreachable | info | | A local-file `source` path does not exist. |

### C: checks

| Code | Name | Default | Fixed | Meaning |
|---|---|---|---|---|
| C001 | check-violated | error | | A `check` is false for every value in tolerance. |
| C002 | check-undecided | warning | | A `check` holds for some values in tolerance and not others. |

### R: read-back and drift

| Code | Name | Default | Fixed | Meaning |
|---|---|---|---|---|
| R001 | drift | error | | Observed value differs from design value at a bound path. Message says which side moved since last sync. |
| R002 | unbound-native | warning | | Native parameter has no binding. |
| R003 | missing-native | error | | Bound path not found in the native file. |
| R004 | regen-failed | error | | The CAD tool failed to rebuild after a push. |

### W: waivers

| Code | Name | Default | Fixed | Meaning |
|---|---|---|---|---|
| W001 | waiver-expired | error | yes | |
| W002 | waiver-stale | warning | yes | Target digest changed since waiver. |
| W003 | waiver-not-allowed | error | yes | Waiver of a fixed code. |
| W004 | waiver-unused | warning | yes | |

## 4. Rule implementation contract

Each rule is a pure function from IR (and, for R rules, bindings plus
observed IR) to diagnostics. Each rule has positive and negative fixtures
under `tests/lints/<code>/`. A rule documents here before it ships; the
code links back with `frob:doc docs/spec/lints.md#...`.
