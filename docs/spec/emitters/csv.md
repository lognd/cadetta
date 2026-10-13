# Emitter: CSV parameter file

Adapter name `csv`, built in, capabilities `emit` and `read`. One row per
bound locator, for spreadsheets and tools that import tables. Default
`out`: `build/<id>.csv`.

## 1. Rows

```
# cadetta emit csv 0.1.0 ir=blake3:<hex> bindings=blake3:<hex>
name,value,unit,note
Plate_Thk,6.35,mm,"1/4in @""ASTM B209"""
PilotBore,12.009,mm,12mm H7 emit=mid
NumBolts,4,,
```

- Line 1 is the header comment (adapter version, IR digest, bindings
  digest); it is excluded from the `sync` no-op comparison
  (adapters.md 2). Reading skips every record that begins with `#`. Only
  the first character of a record is tested: a continuation line of a
  quoted multi-line field that begins with `#` is part of that field's
  data, not a comment, and a `#` inside a name is data.
- Line 2 is the fixed column row `name,value,unit,note`.
- `name` is the bound locator (kind `global`) or the mangled path
  (adapters.md 3.1).
- Only `global` locators in `value` mode are emitted. A `dimension`,
  `property` or `frame` locator, or any locator with `mode = "ref"`,
  bound to a path this run must emit is the error R015 (naming the path
  and the locator), the file is withheld and the exit status is 1; it is
  never skipped silently and never emitted under its locator string.
  "Must emit" is defined as for R014 (emitters/sw-equations.md 1): a
  locator that is filtered out, narrowed away or on a `let` is not
  reported. `read` is unaffected (3).
- `value` is a plain decimal, the emit point (quantities.md 5.6) in the
  locator's `native_unit`, at the declared precision (default 6
  decimals, adapters.md 5), trailing zeros trimmed (a whole number has no decimal point: `10`,
  never `10.0`); not representable
  is Q004. `unit` is empty for Counts, Ratios and Text.
- `note` carries the same content as a trailing comment in other
  emitters (adapters.md 3a), in this order and separated by single spaces: the
  authored literal with tolerance when it differs from the written text,
  `emit=POINT` when the emit point is not `nominal`, the citation as
  written in source (`1/4in emit=mmc @"ASTM B209"`). There is no other
  separator and no other form of the emit token. A `Text` row follows the same rules; as it has no
  tolerance and no emit point, only a citation can appear in its note.
- RFC 4180 quoting, LF line endings, UTF-8 without BOM, final newline.

## 2. Order

Rows in dependency order, ties by canonical path; within one path,
locators in bindings-file order.

## 3. Read

`read` parses the same columns. The `path` is not in the file; it is
resolved through the bindings (adapters.md 3.1). A row whose `name`
matches no binding of this adapter is the error R016 (naming the row
and the file): `read` produces no observed IR for the file and the exit
status is 1. A row is never skipped silently and never matched by
guessing a path. A missing `unit` takes
`--assume-unit` when the run gives one, else the locator's `native_unit`,
else the adapter's declared native unit; the result is recorded as
`unit_assumed`. The explicit flag wins over the configured `native_unit`. Every
locator is `value` mode.

## 4. Obligations

`csv` has no `drive`, so "CAD file reflects design" stays open with the
reason "unverifiable without drive" after `sync` and is reported as O004
(info by default), as for `json` (emitters/json.md 4) and `sw-equations`
(adapters.md 2).
