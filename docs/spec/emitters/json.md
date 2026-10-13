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
- `value` is a decimal string, never a JSON number, so no float rounding
  occurs. It is the emit point (quantities.md 5.6) in the locator's
  `native_unit`, at the declared precision (default 6 decimals,
  adapters.md 5), trailing zeros trimmed. Not representable at that
  precision is Q004. Counts and Ratios are written as integer or decimal
  strings with `unit` omitted; Text is a JSON string with `unit`
  omitted.
- `note` carries what a trailing comment carries in other emitters
  (adapters.md 3a): authored literal with tolerance when it differs from
  the written text, the emit point when not `nominal`, the citation. It
  is omitted when empty.
- JSON is always serialised with 2-space indent, LF line endings, one
  key order as shown, and a final newline.

## 2. Order

`params` in dependency order, ties by canonical path; within one path,
locators in bindings-file order.

## 3. Read

`read` parses the same shape. A missing `unit` takes the locator's
`native_unit` (`unit_assumed`) or `--assume-unit`. `ref` mode does not
exist for this format: every locator is `value`.
