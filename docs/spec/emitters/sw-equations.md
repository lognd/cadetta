# Emitter: SOLIDWORKS linked equations file

Adapter name `sw-equations`, built in, capabilities `emit` and `read`.
The output is a text file a part or assembly links through Equations >
"Link to external file"; SOLIDWORKS re-reads it on rebuild. Derived from
pilot r2-05.

## 1. Lines

```
' cadetta emit sw-equations 0.1.0 ir=blake3:<hex> bindings=blake3:<hex>
' Stock thickness, 1/4 in nominal.
"Plate_Thk" = 6.35mm  ' 1/4in @"ASTM B209"
"D1@Boss-Extrude1@CoverPlate.SLDPRT" = "Plate_Thk"
"PilotBore" = 12.009mm  ' 12mm H7, emit mid
"NumBolts" = 4
```

- Line 1 is the header: adapter version, IR digest, bindings digest.
- **Global lines**, `"NAME" = VALUE`, for each `global` locator. NAME is
  the bound locator, or for an unbound-name global created by the
  emitter, the mangled path with `__` (`"Gasket__width"`); the emitter
  never writes dots in names it creates.
- **Dimension lines**, `"Dn@Feature@File" = REF-OR-VALUE`, for each
  `dimension` locator: `"GLOBAL"` in `ref` mode (which global:
  adapters.md 3), a value in `value` mode.
- **Values** are the emit point (quantities.md 5.6) in the locator's
  `native_unit`, written with the unit suffix (`6.35mm`, `0.25in`), at
  the declared precision (default 6 decimals, adapters.md 5), trailing zeros trimmed. A value
  not representable at that precision is Q004. Counts and Ratios are
  written bare. Text is not emitted to equations (it has no place there).
- **Comments** use `'`. A doc comment becomes the comment lines directly
  above the path's first line, one per source line. The trailing comment
  on a global line carries, in this order and separated by spaces: the
  authored literal with its tolerance or class when its source text
  differs from the written value's text (`1/4in` written as `6.35mm`,
  `12mm H7` written as `12.009mm`; `5mm` written as `5mm` gets none), the emit point when not `nominal`, and the citation.
  Nothing else is ever written in comments.

## 2. Order

Paths in dependency order (a path after everything it reads,
transitively through `let`s and unemitted paths), ties by canonical
path; within one path, global lines in bindings-file order,
then dimension lines in bindings-file order. Emitted values are flat, so
the order only makes the file read top-down; SOLIDWORKS does not need
it.

## 3. Read

`read` parses the same grammar plus anything SOLIDWORKS itself writes
(expressions such as `"A" = "B" * 2`, functions, `IIF`). Expressions
become observed-IR expressions; values without units take the document
unit given by `--assume-unit` or `[[adapter]] native_unit`
(adapters.md 3). Lines it cannot parse are kept with `exact: false` and
the raw text.
