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
# D001 fires at full severity only from this many significant digits.
d001_min_digits = 3
# Round numbers D001 never reports.
common_values = ["1mm", "2mm", "5mm", "10mm", "0.5in", "1in"]
# Per-code severity overrides.
severity = { N002 = "info" }

[obligations.roles.source]
covers = ["ots:*"]
required_for = "check"

[solvers.wall_sizing]
command = ["python", "{script}"]
transport = "stdio"              # stdio | file

[[adapter]]
name = "sw-equations"            # built-in
out = "build/solidworks/equations.txt"
native_unit = "mm"               # document unit for unitless native values (adapters.md 3)

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
| `cadetta check [--release]` | Compile, evaluate, run lints and obligation checks; with adapters that `read`, also drift lints. |
| `cadetta solve [SOLVER...]` | Run solvers and refresh their result files (solvers.md). |
| `cadetta why PATH` / `impact PATH` | Upstream and downstream in the obligation graph. |
| `cadetta impact --swap PART NEW` | Preview a part swap (obligations.md 5). |
| `cadetta compare IFACE` | Implementers of an interface side by side. |
| `cadetta obligations` | Open and stale obligations. |
| `cadetta ack SELECTOR --role R --reason "..."` | Attest at current digests. |
| `cadetta ir [--observed ADAPTER]` | Print design or observed IR as `cadetta.ir/1` JSON. |
| `cadetta emit [ADAPTER...]` | Run `emit` for configured adapters. |
| `cadetta import ADAPTER FILE...` | Read native files, propose canonical names and bindings, scaffold `.cdt`. |
| `cadetta sync [ADAPTER...]` | The full loop (adapters.md 2). |
| `cadetta explain CODE` | Print the catalog entry for a diagnostic. |
| `cadetta units [KIND]` | Print the unit and kind tables. |
