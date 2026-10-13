# The `.cdt` language

Brace-delimited, line-terminated, declarative. A `.cdt` file declares
components and their parameters; there is no control flow.

## 1. Example

```
namespace acme.rover

import std.fits.iso286

/// 5 inch caster wheel, vendor part 4471-B.
ots Wheel {
  source "https://vendor.example/4471-B.pdf#page=2"

  diameter = 5in
  hub_bore = 12mm H7      // tolerance class from the datasheet
  hub_width = 0.75"
}

/// Press fit of the axle into the wheel hub.
fit wheel_axle = H7/p6 on Wheel.hub_bore

model Axle {
  end_diameter = wheel_axle.shaft                was shaft_d
  length = 2 * Wheel.hub_width + Frame.span
  @emit(mid)
  chamfer = 0.5mm +- 0.1mm
}

model Frame {
  span = 300mm
  wall: Length = 3mm
}

assembly Rover {
  part front_left: Wheel
  part front_right: Wheel
  part axle: Axle
  part frame: Frame

  check hub_clearance: frame.span > 2 * front_left.hub_width
}
```

## 2. Lexical structure

- UTF-8 source; cadetta's own repositories are ASCII-only, but user
  files may contain any Unicode in strings and comments.
- Statements end at a newline or `;`. A newline inside `( )` does not
  end a statement.
- `//` line comments; `///` doc comments attach to the next declaration
  and are carried into the IR (and into emitted output where the target
  supports comments).
- Identifiers: `[A-Za-z_][A-Za-z0-9_]*`. Keywords are reserved:
  `namespace import as ots model assembly part fit on param check waive
  because expires was source true false`.
- Quantity literals: quantities.md 2. Strings: `"..."` with `\"` and
  `\\` escapes.

## 3. Declarations

```
file       = [ "namespace" path NL ] { import } { item }
import     = "import" path [ "as" IDENT ] NL
item       = component | fit | waiver
component  = { doc } ( "ots" | "model" | "assembly" ) IDENT "{" { member } "}"
member     = source | param | part | check
source     = "source" STRING NL
param      = { doc } { attr } IDENT [ ":" KIND ] "=" expr
             [ "source" STRING ] { "was" IDENT } NL
part       = "part" IDENT ":" path NL                  // assembly only
fit        = { doc } "fit" IDENT "=" FITCLASS "on" path NL
check      = "check" IDENT ":" expr cmp expr [ STRING ] NL
waiver     = "waive" CODE "on" path "{" "because" STRING NL
             [ "expires" DATE NL ] "}"
attr       = "@" IDENT "(" args ")"
```

### 3.1 Component classes

| Class | Meaning | Values are |
|---|---|---|
| `ots` | Off-the-shelf part. Its parameters are facts about the world. | Literals only (no references). Each value SHOULD have a `source` (S001). |
| `model` | A part we design. | Literals or expressions over any visible parameter. |
| `assembly` | A composition of `part` instances. | May declare its own parameters and `check`s. |

An `ots` parameter is never assigned from an expression: if a vendor
value is derivable, that is a modeling error in the datasheet transcription
(D002). Instances (`part`) do not override parameters in 0.1.0; per-instance
variation is a configuration feature (OQ-4).

### 3.2 Fits

`fit NAME = HOLE/SHAFT on PATH` declares a named fit on the nominal size
of `PATH`. It defines two toleranced quantities, `NAME.hole` and
`NAME.shaft`, from the ISO 286 tables, and records the fit for lints:

- F001 if the classes are unknown, F002 if the size is outside the table.
- F003 if `PATH` itself carries a tolerance that disagrees with the
  fit's hole (or shaft) side.
- The fit's type (clearance, transition, interference) is computed and
  carried in the IR; it is never authored.

### 3.3 Parameters

- `name = value` infers the kind from the value; `name: Kind = value`
  declares it and checks it (quantities.md 4).
- `source "..."` records provenance (URL, drawing number, file anchor).
- `was old_name` records a rename. References to `old_name` resolve with
  N004 (`renamed-reference`, warning) until updated; adapters use `was`
  to keep native bindings stable across renames.
- Attributes: `@emit(nominal|mid|mmc|lmc)`, `@display(unit)`,
  `@native(adapter, "locator")` (pin a binding inline; adapters.md 3).

## 4. Names and resolution

- Canonical path of a parameter: `namespace::Component.param`; of an
  instance parameter: `namespace::Assembly/part.param` (ir.md 2).
- Inside a component, a bare name resolves to the component's own
  parameter. `Other.param` resolves a component in the same namespace,
  then imported namespaces. In an assembly, `part.param` resolves an
  instance.
- Reference cycles are D003 (error). Order of declaration is irrelevant.
- Naming canon (N001): namespaces `lower.dotted`, components
  `PascalCase`, parameters, fits, parts and checks `snake_case`.

## 5. Expressions

`+ - * / ^`, unary `-`, parentheses, references, quantity literals, and
the functions `min max abs sqrt sin cos tan atan2 round_to(x, step)`.
Kind and tolerance rules are in quantities.md 4-5. `round_to` is the
only rounding operation and is exact for exact inputs.

## 6. Checks

`check NAME: lhs CMP rhs ["message"]` with `CMP` one of `< <= > >= ==`.
Evaluation is three-valued over tolerances (quantities.md 5.3):
`violates` is C001 (error), `undecided` is C002 (warning).

## 7. Formatting

`cadetta fmt` produces one canonical layout: two-space indent, one
statement per line, aligned `=` within a block, units preserved as
authored. Formatting never changes the IR digest.

## Open questions

- OQ-4: Configurations (SOLIDWORKS configurations, design tables, part
  families). Probably `variant` blocks on components; out of 0.1.0.
- OQ-5: File extension. `.cdt` is proposed; it collides with a few
  obscure formats.
- OQ-6: Features named by function (`bore`, `seat`, `datum`) as a layer
  above raw parameters, as in lithos hematite. Out of 0.1.0.
