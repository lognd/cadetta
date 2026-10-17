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

## Follow-up live run (~PF7XCNP, same machine)

Driver: `tools/pf7x/` (a C# net48 exe; PowerShell cannot cast the COM
object to the interop `ISldWorks`). Log: `evidence/pf7x-live/pf7x-live.log`.
It calls `PushService` over `SwComPushDocument`, the same path the menu
Link verb uses, not the menu clicks themselves. Open documents were logged
before and after each step: none left open.

| Item | Result |
| --- | --- |
| pattern count link (`D1@LPattern1` to global `HoleCount`) | linked, global = 5, unchanged push skipped |
| tolerance bilateral and fit on `D1@Boss-Extrude1` | passed |
| live rebuild-failure restore (`D1@Shell1` pushed to 50 mm) | passed: R004, global and dimension back to 1, link kept |

Findings (not fixed here, filed as tickets):

- A pattern count reads back x1000 and a re-push of it writes 4000: every
  non-angle dimension is scaled by the length factor in `SwComPushDocument`.
- The push-path R004 message does not name the failing feature.

## Datum frame verb: dropped

No menu verb for datum frame creation is added. Frames are created by
`sync` through the `create_frames` push, which was verified live
(`tests/live/frames-nova-live.log`); a second entry point would duplicate it.

## NOT exercised

- The r3-05 pilot itself (stand-in used again).
- The true menu-click path (select dimension, Link verb) for a pattern
  count and a tolerance; only the service path beneath it ran.
- `import --update` on a toleranced or fit value (refused by design).
- Moving `D1@Shell1` through the API: it does not take effect, so `wall`
  stayed at 1 mm.

## Findings left as tickets

Filed as separate tickets (see the ticket Done report): `cadetta names`
opening the project file in the running SOLIDWORKS; OpenDoc6 relative-path
failure in `import`; read-back mismatch when CAD carries a tolerance the
design lacks; `read_current` returning None for skipped statuses; a failed
atomic restore leaving a half-pushed document; a transient
RPC_E_SERVERFAULT on the first read.
