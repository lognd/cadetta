# 1scpzk7-live: SOLIDWORKS count-link and import live check

Rerun of the w5mb-live run5 count-link/import scenario on nova (SOLIDWORKS 2025,
Windows 11 ARM64) with a build containing the dimensionless fixes
68680ce38 (SetGlobal with DimensionUnitKind/GlobalLiteral) and c3db81910
(import --update drops the assumed unit from a bare count).

- `run.log`: full transcript. `run.cmd`: the nova driver script.
- Driver: `adapters/solidworks/tools/w5mb` (new `link` command).

Result: the count dimension `D1@LPattern1` has kind=Count value=4, the global
`bolt_count` is created as `4` (never `4mm`), sync baseline is `4 -> 4`,
and `import --update` after a CAD edit changes only `length` (`120mm` ->
`130mm`); lib.cdt keeps `bolt_count: Count = 4`. A final sync is a no-op.
SOLIDWORKS document count was 0 before and after every step; the app was
not restarted.

Caveat: the add-in menu was not exercised (swapping the loaded add-in dll
needs a SOLIDWORKS restart). The link was driven out of process through the
production `LinkCommands.Link` and `SwComPushDocument` code, from the same
Core and Com dlls the add-in uses. The pilot_bore link also logs the
expected R006 "replaces native expression" warning on sync push.
