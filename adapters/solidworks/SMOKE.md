# SOLIDWORKS release smoke (nova, SOLIDWORKS 2025 rev 33.3.0)

Ticket ~NEHY9X7. Machine: nova (Windows ARM64, x64 SOLIDWORKS under
emulation). Evidence: `evidence/nehy-live/`.

## Stand-in note

The r3-05 pilot source was not available on nova. A stand-in project was
used instead: a one-part rig `live.rig::Rig` with a `Plate` part
(`C:\src\nehy\proj`), `thickness` bound to `D1@Boss-Extrude1` and `wall`
bound to `D1@Shell1`. Results below are for the stand-in, not for r3-05.

## Results

| Criterion | Result | Evidence |
| --- | --- | --- |
| (a) emit, read, push, regen round trip | passed: push, regen, read-back, save, then no-op | `drive-sync-pushed-and-noop.txt`, `drive-converged-and-noop.txt` |
| (b) attach, sync, save | passed against real SOLIDWORKS | `addin-connect.log`, `drive-sync-pushed-and-noop.txt` |
| (b) refused write blocks save | passed (R010 and R009) | `drive-refused-write.txt` |
| (c) sync, cad-moved R001, import --update, no-op | passed on a plain `thickness` | `sync-sequence.txt`, `drive-cad-moved-import-update-noop.txt` |
| (c) add-in Link, Unlink, Preview, Why, Sync | exercised live | `addin-buttons.log` |
| (c) add-in AssignArray | exercised live: assigned Plate-1..3 to feet[0..2] | `addin-buttons.log` |
| toleranced value on cad-moved | skipped by design ("edit by hand") | `drive-cad-moved-tolerance-skip.txt` |

## Defects found and fixed here

- Bare observed-IR file names did not resolve: the drive server now
  resolves them under the working directory (`FindUnderCwd`).
- Save digest hit a sharing violation: `DiskFileReader` opens with
  `FileShare.ReadWrite | FileShare.Delete`.
- `ArrayPrefix` refused leaf rows; it now accepts them.
- Add-in: re-entrancy guard around CLI calls, deferred pane refresh, and a
  dependency resolver for the interop assemblies.
- Drive protocol `open`, `save`, `close` are now served (`DriveSessionTests`).

## NOT exercised

- The r3-05 pilot itself (stand-in used).
- A pattern count link.
- A tolerance link created through the add-in menu. The CAD tolerance was
  only observed, and cleared with the test control tool.
- Datum frame creation: the add-in has no menu verb for it.
- A live rebuild-failure restore (unit tests only).
- Moving `D1@Shell1` through the API: it does not take effect, so `wall`
  stayed at 1 mm.
- `import --update` on a toleranced or fit value (refused by design).

## Findings left as tickets

Filed as separate tickets (see the ticket Done report): `cadetta names`
opening the project file in the running SOLIDWORKS; OpenDoc6 relative-path
failure in `import`; read-back mismatch when CAD carries a tolerance the
design lacks; `read_current` returning None for skipped statuses; a failed
atomic restore leaving a half-pushed document; a transient
RPC_E_SERVERFAULT on the first read.
