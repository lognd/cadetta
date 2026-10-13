# Adapters

An adapter connects the IR to one CAD tool or file format. Cadetta has no
favourite CAD tool: every target is an adapter, and the core never links
a vendor SDK.

## 1. Capabilities

Each adapter declares which of three capabilities it implements.

| Capability | Direction | What it does | Removes the manual step of |
|---|---|---|---|
| `emit` | IR -> file | Writes a parameter file the CAD tool links to or imports. | retyping values |
| `read` | file -> observed IR | Extracts parameters (global variables, equations, dimension values, custom properties) from native files. | transcribing existing designs |
| `drive` | IR <-> live tool | Pushes values into an open or headless session, triggers regeneration, and reads results back. | open, import, rebuild, check |

`drive` implies the adapter can report regeneration failures (rebuild
errors, failed features) as diagnostics (R004).

## 2. The loop

```
cadetta sync = compile -> lint -> push changed params -> regen -> read back -> drift lint
```

`sync` is idempotent: when no digest changed and read-back matches, it
writes nothing and triggers no regen. Without `drive`, `sync` degrades to
`emit` plus instructions for the user's one manual step (relinking or
reloading), and `read` on the next run detects drift.

## 3. Bindings

A binding maps a canonical path to an adapter's native locator, e.g.
`acme.rover::Axle.end_diameter` to the SOLIDWORKS global variable
`"Axle_end_diameter"` or to the dimension `"D1@Boss-Extrude1@Axle.SLDPRT"`.

- Bindings live in `bindings/<adapter>.toml`, committed, one table per
  canonical path. `@native(...)` in source pins one inline.
- `cadetta import` proposes bindings and canonical names from observed
  IR; the user accepts them, which writes the bindings file and scaffolds
  `.cdt` source. Nobody types the values.
- Unbound native parameters are R002; bound paths missing natively are
  R003. Renames (`was`) carry bindings forward automatically.
- Each adapter defines a deterministic default name mangling for emitted
  parameters (`Axle.end_diameter` -> `Axle_end_diameter` for tools with
  flat namespaces) so most bindings never need to be written.

## 4. Contract every adapter MUST meet

1. **Emitters decide nothing.** Output is a pure function of (IR digest,
   bindings, adapter version, adapter config). Any choice an adapter
   seems to need (which tolerance point, which unit) is decided upstream
   by policy and arrives in the request.
2. **Deterministic bytes.** Same inputs, same output bytes. Golden-file
   fixtures for write-only formats; round-trip fixtures (emit, read,
   compare IR values) for formats the adapter can also read.
3. **Exactness is reported, not hidden.** Converting a rational to the
   target's native number format and unit happens in the adapter at a
   declared precision; if the value is not representable within that
   precision, the adapter returns Q004 with the error, never rounds
   silently.
4. **Read is lossless or says so.** A value the adapter cannot extract
   faithfully (driven dimension, expression it cannot parse) appears in
   observed IR with `exact: false` and a reason, not as a guess.
5. **No source of truth outside the IR.** An adapter never writes back
   into `.cdt` source; `import` produces a proposal the user accepts.

## 5. Protocol

Built-in adapters run in process (Rust). External adapters are separate
executables, written in whatever language reaches the CAD API (C# for
SOLIDWORKS COM, Python for FreeCAD and Fusion), speaking JSON-RPC 2.0
over stdio:

| Method | Params | Result |
|---|---|---|
| `initialize` | protocol version, client info | name, version, capabilities, native units, precision, platforms |
| `emit` | IR (design), bindings, policy | files written (paths, digests), diagnostics |
| `read` | file paths | observed IR, diagnostics |
| `push` | changed params (path, value, locator) | diagnostics |
| `regen` | targets | per-target status, rebuild diagnostics |
| `shutdown` | - | - |

Adapters are declared in `cadetta.toml` (`[[adapter]]`, config-cli.md);
the protocol schema lives in `docs/schemas/` beside the IR schema.

## 6. Target matrix

Planned support, not a 0.1.0 promise beyond the rows marked 0.1.0.

| Target | Kind | emit | read | drive | Mechanism |
|---|---|---|---|---|---|
| JSON | built-in | 0.1.0 | 0.1.0 | - | the IR itself |
| CSV | built-in | 0.1.0 | 0.1.0 | - | name, value, unit rows |
| SOLIDWORKS equations file | built-in | 0.1.0 | 0.1.0 | - | text file the part links via "Link to external file" |
| FreeCAD | external, Python | 0.1.0 | 0.1.0 | 0.1.0 | Spreadsheet aliases; headless `freecadcmd` for CI |
| SOLIDWORKS | external, C# | - | later | later | COM API: global variables, dimensions, rebuild |
| Onshape | external | - | later | later | REST: variable studios, part studio variables |
| Fusion | external, Python | - | later | later | user parameters API |
| Inventor | external | - | later | later | parameters API |
| OpenSCAD / CadQuery | built-in | later | - | - | `-D` defines / Python constants |
| KiCad | - | out | out | out | electrical CAD is out of 0.1.0 |

FreeCAD is the reference external adapter because it is the only target
that can exercise `read` and `drive` in CI without a license or Windows.

## Open questions

- OQ-7: Adapter discovery and versioning (PATH lookup vs a lockfile of
  adapter versions). Recommendation: `cadetta.toml` pins command and
  version; `initialize` must match.
- OQ-8: How `drive` handles a CAD session the user has open with unsaved
  changes. Recommendation: refuse and report, never save on the user's
  behalf.
