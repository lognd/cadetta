# Emitter: JSON parameter file

Adapter name `json`, built in, capabilities `emit` and `read`. A flat
parameter list for tools that load JSON (scripts, CadQuery, web
configurators). Not the IR: for the IR use `cadetta ir`. Default `out`:
`build/<id>.json`.

## 1. Shape

```json
{
  "schema": "cadetta.params/1",
  "header": { "adapter": "json 0.1.0", "ir": "blake3:<hex>", "bindings": "blake3:<hex>" },
  "params": [
    { "name": "Plate_Thk", "path": "acme.gearbox::CoverPlate.thickness",
      "value": "6.35", "unit": "mm", "note": "1/4in @\"ASTM B209\"" }
  ]
}
```

- `header` carries adapter version, IR digest and bindings digest; it is
  excluded from the `sync` no-op comparison (adapters.md 2).
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
- `value` is a decimal string, never a JSON number, so no float rounding
  occurs. It is the emit point (quantities.md 5.6) in the locator's
  `native_unit`, at the declared precision (default 6 decimals,
  adapters.md 5), trailing zeros trimmed (a whole number has no decimal point: `10`,
  never `10.0`). Not representable at that
  precision is Q004. Counts and Ratios are written as integer or decimal
  strings with `unit` omitted; Text is a JSON string with `unit`
  omitted.
- `note` carries what a trailing comment carries in other emitters
  (adapters.md 3a), in this order and separated by single spaces: authored
  literal with tolerance when it differs from the written text,
  `emit=POINT` when the emit point is not `nominal`, the citation as
  written in source. It
  is omitted when empty. A `Text` parameter follows the same rules; as it
  has no tolerance and no emit point, only a citation can appear in its
  note.
- JSON is always serialised with 2-space indent, LF line endings, one
  key order as shown, and a final newline.

## 2. Order

`params` in dependency order, ties by canonical path; within one path,
locators in bindings-file order.

## 3. Read

`read` parses the same shape. A missing `unit` takes `--assume-unit` when the
run gives one, else the locator's `native_unit`, else the adapter's
declared native unit; the result is recorded as `unit_assumed`. The explicit
flag wins over the configured `native_unit`. `ref` mode does not
exist for this format: every locator is `value`. Reading matches a
parameter by `name` whatever the locator's kind, so R015 (1) is an emit
rule only.

## 4. Obligations

`json` has no `drive`, so it cannot discharge "CAD file reflects design"
(obligations.md 2): after `sync` the obligation stays open with the
reason "unverifiable without drive" and is reported as O004 (info by
default), exactly as for `sw-equations` (adapters.md 2). A `drive`
adapter covering the same file discharges it; so does `ack`.
