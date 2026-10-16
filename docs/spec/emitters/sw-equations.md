# Emitter: SOLIDWORKS linked equations file

Adapter name `sw-equations`, built in, capabilities `emit` and `read`
(no `drive`). Default `out`: `build/solidworks/<id>.txt`.
The output is a text file a part or assembly links through Equations >
"Link to external file"; SOLIDWORKS re-reads it on rebuild. Derived from
pilot r2-05.

## 1. Lines

```
' cadetta emit sw-equations 0.1.0 ir=blake3:<hex> bindings=blake3:<hex>
' Stock thickness, 1/4 in nominal.
"Plate_Thk" = 6.35mm  ' 1/4in @"ASTM B209"
"D1@Boss-Extrude1@CoverPlate.SLDPRT" = "Plate_Thk"
"PilotBore" = 12.009mm  ' 12mm H7 emit=mid
"NumBolts" = 4
```

- Line 1 is the header: adapter version, IR digest, bindings digest. It
  is excluded from the `sync` no-op comparison (adapters.md 2).
- **Global lines**, `"NAME" = VALUE`, for each `global` locator. NAME is
  the bound locator, or for an unbound-name global created by the
  emitter, the mangled path (`"Gasket__width"`, adapters.md 3.1); the
  emitter never writes dots in names it creates. A `ref` dimension with
  no `ref_to`, no bound global and no pure-alias referent makes the
  emitter create this global (adapters.md 3); it is a global line of the
  path, takes the dimension locator's `native_unit` (else the adapter's
  declared native unit) and carries the path's value in that unit.
  Under `[[adapter]] bind = "scope"` (adapters.md 3.5) every in-scope
  path the format can express has a global line under its created
  name, so dimensions can be linked to it inside SOLIDWORKS
  (solidworks.md 2.4); this emitter cannot read parts, so it discovers
  no native links.
- **Dimension lines**, `"Dn@Feature@File" = REF-OR-VALUE`, for each
  `dimension` locator: `"GLOBAL"` in `ref` mode (which global:
  adapters.md 3), a value in `value` mode.
- **Values** are the emit point (quantities.md 5.6) in the locator's
  `native_unit`, written with the unit suffix (`6.35mm`, `0.25in`), at
  the declared precision (default 6 decimals, adapters.md 5), trailing
  zeros trimmed. A value not representable at that precision is Q004. Counts and Ratios are
  written bare. An equations file cannot express `Text`, so a bound `Text`
  parameter that this run must emit is the error R014 (naming the path and
  the locator), the file is withheld and the exit status is 1; nothing is
  skipped silently and no placeholder is written. "Must emit" means the
  path is bound by a locator of this adapter, passes the adapter's
  `modules`/`paths` filters and the run's `--module`/glob narrowing, is not
  a `let` and (for an interface-typed slot) belongs to an active alternate
  (adapters.md 3.3). A `Text` parameter that fails any of those, or that
  has no binding, is not emitted and is not an error. This holds whatever
  the locator kind (`global` or `dimension`).
- **Only `global` and `dimension` locators are emitted.** An equations
  file has no syntax for a custom property, a coordinate system or a
  component placement, so a `property`, `frame` or `instance` locator
  (adapters.md 3) bound to a path this run must emit is the error R015
  (naming the path and the locator), the file is withheld and the exit
  status is 1; it is never skipped silently and never emitted under its
  locator string. "Must emit" is defined as for R014 above; a locator
  that is filtered out, narrowed away or on a `let` is not reported.
  Those kinds belong to the COM `solidworks` adapter (solidworks.md).
  `read` is unaffected: it reads only equations (3).
- **Comments** use `'`. A doc comment becomes the comment lines directly
  above the path's first line, one per source line. The trailing comment
  on a global line carries, in this order and separated by spaces: the
  authored literal with its tolerance or class when its source text
  differs from the written value's text (`1/4in` written as `6.35mm`,
  `12mm H7` written as `12.009mm`; `5mm` written as `5mm` gets none),
  then `emit=POINT` when the emit point is not `nominal`, then the
  citation as written in source.
  Nothing else is ever written in comments.

## 2. Order

Paths in dependency order, computed as a topological sort that always
takes, among the paths whose dependencies are all placed, the smallest
canonical path (byte order); dependencies count transitively through
`let`s and unemitted paths; within one path, global lines in bindings-file order,
then dimension lines in bindings-file order. Emitted values are flat, so
the order only makes the file read top-down; SOLIDWORKS does not need
it.

## 3. Read

`read` parses the same grammar plus anything SOLIDWORKS itself writes
(expressions such as `"A" = "B" * 2`, the equation functions `sin`, `cos`,
`tan`, `sqr`, `abs`, `iif` and the rest of SOLIDWORKS' list). A call to any
other function name is not parseable. Expressions
become observed-IR expressions; values without units take the document
unit given by `--assume-unit` or `[[adapter]] native_unit`
(adapters.md 3). Lines it cannot parse are kept with `exact: false` and
the raw text.

`read` exists chiefly to import an equations list exported from
SOLIDWORKS (Equations dialog > Export), which is how a user without COM
gets observed IR for `import` (adapters.md 7). Because a linked equations
file is not editable in SOLIDWORKS, reading the file cadetta wrote
reports only what cadetta wrote.

## 4. SOLIDWORKS naming

Both SOLIDWORKS adapters (`sw-equations` and the COM `solidworks`
adapter) MUST follow these rules, so switching adapters never renames
anything in a part, and may share one bindings file
(`[[adapter]] bindings = "bindings/solidworks.toml"`, adapters.md 3).

- **Created names** follow adapters.md 3.1: `__` always, never a dot.
- **Dimension locators**, by context:

  | Context | Spelling | Extra field |
  |---|---|---|
  | part | `Dn@Feature@File.SLDPRT` (`Dn@Sketch1@File.SLDPRT` for a sketch dimension) | - |
  | assembly (a mate distance, an assembly-level feature) | `Dn@Feature@File.SLDASM` | - |
  | component inside an assembly | `Dn@Feature@File.SLDPRT` | `component = "wheel<2>"` |

  `component` is the instance name as the feature tree shows it. The
  COM adapter derives the assembly-context spelling from it. A binding on
  one instance path (`Rover/wheel[2].hub_bore`) addresses the component
  named by `component`; a `[*]` binding without `component` addresses
  the part file itself (all instances share its dimensions).
- Coordinates bound with `sign = -1` are negated at write and read
  (geometry.md 6).

## 5. Obligations

This adapter never discharges "CAD file reflects design": it has no
`drive`, so cadetta cannot see what the part holds. The obligation stays
open with the reason "unverifiable without drive" (adapters.md 2),
unless a `drive` adapter covers the same file or the user attests it.
