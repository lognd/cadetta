# Configuration and command line

## 1. `cadetta.toml`

`cadetta init` writes a complete file with every knob at its default and
a comment per knob.

```toml
[project]
namespace = "acme.rover"
sources = ["cad/**/*.cdt"]

[units]
# Display unit per kind; authored units win when unset.
display = { Length = "mm", Angle = "deg", Mass = "kg" }

[emit]
# Tolerance point written to CAD: nominal | mid | mmc | lmc.
point = "nominal"

[lint]
fail_on = "error"
# Max significant decimals in the display unit before M001 fires.
precision = { Length = 2, Angle = 1 }
# Round numbers D001 downgrades to info.
common_values = ["1mm", "2mm", "5mm", "10mm", "0.5in", "1in"]
# Per-code severity overrides.
severity = { N002 = "info" }

[[adapter]]
name = "sw-equations"            # built-in
out = "build/solidworks/equations.txt"

[[adapter]]
name = "freecad"                 # external
command = ["cadetta-freecad"]
version = "0.1"
files = ["cad/freecad/*.FCStd"]
```

## 2. Commands

All commands accept `--format text|json` (json is the default when
stdout is not a terminal), `-v`, `--cwd`. Exit codes: 0 ok, 1 diagnostics
at or above `fail_on`, 2 usage or I/O error.

| Command | Does |
|---|---|
| `cadetta init` | Write `cadetta.toml` and a hello-world `.cdt`. Safe to repeat. |
| `cadetta fmt [--check]` | Canonical formatting (language.md 7). |
| `cadetta check` | Compile, evaluate, run lints; with adapters that `read`, also drift lints. |
| `cadetta ir [--observed ADAPTER]` | Print design or observed IR as `cadetta.ir/1` JSON. |
| `cadetta emit [ADAPTER...]` | Run `emit` for configured adapters. |
| `cadetta import ADAPTER FILE...` | Read native files, propose canonical names and bindings, scaffold `.cdt`. |
| `cadetta sync [ADAPTER...]` | The full loop (adapters.md 2). |
| `cadetta explain CODE` | Print the catalog entry for a diagnostic. |
| `cadetta units [KIND]` | Print the unit and kind tables. |
