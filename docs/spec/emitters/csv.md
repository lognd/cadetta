# Emitter: CSV parameter file

Adapter name `csv`, built in, capabilities `emit` and `read`. One row per
bound locator, for spreadsheets and tools that import tables. Default
`out`: `build/<id>.csv`.

## 1. Rows

```
# cadetta emit csv 0.1.0 ir=blake3:<hex> bindings=blake3:<hex>
name,value,unit,note
Plate_Thk,6.35,mm,"1/4in @""ASTM B209"""
PilotBore,12.009,mm,12mm H7; emit mid
NumBolts,4,,
```

- Line 1 is the header comment (adapter version, IR digest, bindings
  digest); it is excluded from the `sync` no-op comparison
  (adapters.md 2). Reading skips every line that begins with `#`.
- Line 2 is the fixed column row `name,value,unit,note`.
- `name` is the bound locator (kind `global`) or the mangled path
  (adapters.md 3.1).
- `value` is a plain decimal, the emit point (quantities.md 5.6) in the
  locator's `native_unit`, at the declared precision (default 6
  decimals, adapters.md 5), trailing zeros trimmed; not representable
  is Q004. `unit` is empty for Counts, Ratios and Text.
- `note` carries the same content as a trailing comment in other
  emitters (adapters.md 3a), joined with `; `: authored literal with
  tolerance when it differs from the written text, the emit point when
  not `nominal`, the citation.
- RFC 4180 quoting, LF line endings, UTF-8 without BOM, final newline.

## 2. Order

Rows in dependency order, ties by canonical path; within one path,
locators in bindings-file order.

## 3. Read

`read` parses the same columns. The `path` is not in the file; it is
resolved through the bindings (adapters.md 3.1). A missing `unit` takes
the locator's `native_unit` (`unit_assumed`) or `--assume-unit`. Every
locator is `value` mode.
