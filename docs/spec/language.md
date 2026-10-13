# The `.cdt` language

Brace-delimited, line-terminated, declarative and total: every program
terminates and its dependency graph is known without evaluating it. The
expression language borrows Rust's flavour (expressions everywhere,
`fn`, `enum`, `match`, `#[attributes]`) without mutation, loops,
recursion, traits or closures.

## 1. Example

```
namespace acme.rover

use std.fits::iso286
use std.fasteners::{MetricThread, ScrewJoint}
use std.materials::Material

cite colson   = "https://colson.example/2-series.pdf"
cite customer = "docs/requirements.md"
cite vendor   = "https://vendor.example/4471-B.pdf"

/// What the rover needs from a caster wheel, whoever makes it.
interface Caster {
  diameter: Length
  hub_bore: Length
  hub_width: Length
  load_rating >= 400N                     // a bound every implementer must meet
}

#[lifecycle(eol, "2027-06")]
ots Vendor4471B: Caster @vendor {
  diameter    = 5in
  hub_bore    = 12mm [+0.018mm -0]        @vendor:"p.2, table 1"
  hub_width   = 3/4in
  load_rating = 450N
  unit_cost   = 11.20USD
  lead_time   = 2wk
}

ots Colson2Series: Caster @colson {
  diameter    = 5in
  hub_bore    = 12mm H8
  hub_width   = 7/8in
  load_rating = 540N
  unit_cost   = 13.90USD
  lead_time   = 4wk
}

/// Axle for a pair of wheels; takes the wheel as an argument so a swap is one line.
model Axle(wheel: Caster, span: Length) {
  #![emit(nominal), display(mm)]

  fit seat     = p6 on wheel.hub_bore              // shaft side; hole side is the vendor's bore
  end_diameter = seat.shaft
  length       = 2 * wheel.hub_width + span        was Globals.axle_len
  chamfer      = 0.5mm [+-0.1mm]                   #[emit(mid)]
}

model Bracket {
  thickness = 4mm
}

model Frame {
  span   = 300mm
  wall: Length = if material == Material::Al6061T6 { 3mm } else { 4mm }
  material = Material::Al6061T6
}

assembly Rover {
  part wheel[4]: Caster = Vendor4471B alt [Colson2Series]
  part frame: Frame
  part bracket[2]: Bracket
  part axle[2]: Axle(wheel = wheel[0], span = frame.span)

  require mass_budget: mass?(self) <= 6kg  @customer:"REQ-12"
  check load_ok for w in wheel: w.load_rating * count(wheel) >= mass?(self) * 1gn * 2

  // "Screw here": hole sizes on both parts, tap drill and screw length come from std (stdlib.md 3).
  part bracket_screws[4]: ScrewJoint(thread = M5, clamp = bracket[0].thickness, into = frame.material)
}
```

## 2. Lexical structure

- UTF-8 source. Statements end at a newline or `;`; a newline inside
  `( ) [ ]` does not end one.
- `//` line comments; `///` doc comments attach to the next item and are
  carried into the IR and into emitted output where supported.
- Identifiers `[A-Za-z_][A-Za-z0-9_]*`. Reserved keywords: `namespace
  use as cite interface ots model assembly process table enum fn let if
  else match part alt fit on solver in out require check for satisfies
  waive because expires was unit kind self true false`.
- Quantity literals: quantities.md 2. Strings `"..."` with `\"`, `\\`.
- `@` introduces a citation (3.9). `#[...]` and `#![...]` are attributes
  (3.11). A `:` directly after a citation starts its locator string.

## 3. Items

```
file      = [ "namespace" path ] { use | item }
use       = "use" nspath "::" ( IDENT | "{" IDENT { "," IDENT } "}" ) [ "as" IDENT ]
nspath    = IDENT { "." IDENT }                  // namespaces are dotted; "::" enters one
item      = { doc } { attr } ( cite | interface | component | process | table
            | enum | fn | fit | solver | require | check | waive | unit | kind )
component = ( "ots" | "model" | "assembly" ) IDENT [ params ] [ ":" iface_list ]
            [ citation ] "{" { inner_attr } { member } "}"
params    = "(" pdecl { "," pdecl } ")"
pdecl     = IDENT ":" type [ "=" expr ]              // a default makes the argument optional
member    = { doc } { attr } ( param | let | bound | part | fit | check | require
            | solver ) [ trailing_attr ]
let       = "let" IDENT [ ":" type ] "=" expr        // analysis value (quantities.md 5.3)
param     = IDENT [ ":" type ] "=" expr [ citation ] { "was" path }
bound     = IDENT ( "<=" | ">=" ) expr | IDENT "in" expr ".." expr
part      = "part" IDENT [ "[" expr "]" ] ":" type [ "=" ctor [ "alt" "[" ctor { "," ctor } "]" ] ]
ctor      = path [ "(" args ")" ]
check     = "check" IDENT [ "for" IDENT "in" expr ] ":" expr [ STRING ]
process   = "process" IDENT [ citation ] "{" { param | rule } "}"
rule      = "rule" IDENT "(" pdecl ")" "->" type "=" expr
table     = "table" IDENT [ citation ] "{" "key" pdecl [ "cols" pdecl { "," pdecl } ]
            { expr [ "=>" expr { "," expr } ] } "}"   // no "=>": a key-only row
```

### 3.1 Component classes

| Class | Meaning | May contain |
|---|---|---|
| `ots` | Off-the-shelf: facts about a part someone else makes. | Literals and expressions over its own parameters (a drawing states pitch and positions). Every value is cited (S001). Commercial data (`unit_cost`, `lead_time`, `moq`, `supplier`) is ordinary parameters. |
| `model` | A part we design. | Anything. |
| `assembly` | A composition of `part` instances. | Parameters, `let`s, parts, fits, checks, requires, solvers. |

Components may take parameters (`ots Iso4762(thread: MetricThread,
length: Length)`, `model Axle(wheel: Caster)`), optionally with defaults.
Arguments are given at the `part` site. A parameterized `ots` is a part
family; its values are typically table lookups (3.6).

`Type.member` reads a member of a parameterized component without an
instance when that member does not depend on any parameter
(`TrScrew.cut_increment`), so an argument can be computed from the
family itself: `part screw: TrScrew(length = ceil_to(min_len,
TrScrew.cut_increment))`. Reading a member that does depend on a
parameter this way is an error naming the parameter.

A component with no parameters read by type name (`Layout.ring_dia`,
`CoverPlate.thickness`) is one implicit instance, shared by every such
read anywhere in the project: one source variable per toleranced member
(quantities.md 5.4), distinct from every `part` instance of the type.
Reading a type by name inside an assembly that also has a `part` of
that type is N006, which suggests the instance.

Enum arguments may be written as bare variants when the parameter's type
is known (`Iso4762(M5, 25mm)`); elsewhere they are qualified
(`MetricThread::M5`).

### 3.2 Interfaces and part swapping

An `interface` names what a design needs from a part: members with kinds,
and bounds every implementer must meet. A component implements it with
`: Caster`; a missing member is I001 (error); a violated bound is C001
against that implementer.

A `part` typed by an interface names a primary implementer and,
optionally, approved alternates: `part wheel[4]: Caster = Vendor4471B
alt [Colson2Series]`. Code that reads `wheel.hub_bore` depends on the
interface only, so:

- **Swapping is one edit**: change the primary. Nothing else is touched.
- **Alternates are verified continuously** by variant evaluation: for
  each part slot and each of its alternates, the alternate replaces the
  primary in every instance of that slot (all four wheels together), all
  other slots keep their primaries, and everything downstream is
  re-evaluated: derived values, part arguments, fits, checks, requires
  and roll-ups. Slots vary one at a time, never as a cross product, so
  the cost is linear in the number of alternates. Values that differ
  from the primary's are kept per variant in the IR (ir.md 3); a failure
  names the alternate (C001 `for alt Colson2Series`). An evaluation
  error under a variant (a table lookup that misses, a failed `as`)
  makes that alternate non-viable, with the error as the reason; it
  never fails the run.
- **Viable** means no check, fit, bound or require violates under the
  variant. Undecided results leave it viable with I003. A non-viable
  alternate stays listed (I002); a waiver may be scoped to one
  alternate (`waive C002 on Rover.load_ok for alt Colson2Series`), but a
  violated `require` makes an alternate non-viable whatever is waived.
- **Promoting** an alternate is still the one edit: it becomes the
  primary and leaves the `alt` list. The old primary stays approved only
  if the edit puts it in the list; `impact --swap` prints the suggested
  line with the old primary appended.
- **Reasons to swap are data, not syntax**: cost, lead time, supply
  risk, weight, performance, compliance or end of life. They are
  parameters and attributes on the implementers, compared by `cadetta
  compare Caster` and previewed by `cadetta impact --swap` (obligations.md
  5). `#[lifecycle(active|nrnd|eol|obsolete, "date")]` feeds L001/L002;
  the date is when the status takes effect (for `eol`, the last-time
  buy), and L001 becomes an error after it. For `active` the date is
  when the status was last verified.

### 3.3 Parameters

- `name = value` infers the kind; `name: Kind = value` checks it.
- `let name = expr` declares an analysis value: readable like a
  parameter (including as a part argument and from other components),
  kept with its full analysis tolerance, never emitted or bound
  (quantities.md 5.3). Stack-ups and other quantities nobody makes are
  `let`.
- A trailing citation records provenance (3.9).
- `was path` records a rename or move (`was Globals.plate_thk`).
  References to the old path resolve with N004; bindings follow it.
- Tolerance semantics on assignment: quantities.md 5.3.
- Non-quantity kinds: `Text` (`designation = "1/4-20 UNC-2A"`), enums,
  `Bool`, `Point2`, `Point3`, fixed arrays `[T; N]`, and `Table<K>`, a
  reference to any table keyed by `K` whose columns are read as usual
  (`policy.lengths.at_least(x).key`). Columns a reader uses must exist
  in the table passed (checked at the use site).

### 3.4 Bounds

`name <= expr`, `name >= expr`, `name in lo..hi` declare a requirement on
a value rather than a value (quantities.md 5.7). In an `ots` a bound
records a vendor limit or recommendation (`abutment_dia >= 9.6mm`); in an
`interface` it binds every implementer; in a `model` it is a design
requirement others are checked against with `satisfies`. An interface
that needs to expose a vendor limit declares it as a plain member
(`shaft_abutment_min: Length`); each implementer sets it from its
datasheet bound, and the user's design checks against it.

### 3.5 Parts

`part name: Type`, `part name[N]: Type`, `part name: Type(args)`. `N` is
a Count expression over parameters and `count()` only, evaluated before
any member, so the part graph is known without evaluating the design;
D003 covers it. Instances are distinct: each toleranced parameter of
each instance is an independent source variable (quantities.md 5.4).
`count(name)` is a Count; `name[i]` an instance; `for x in name`
iterates in checks. Every instance of an array has the same arguments,
so `name.member` (no index) reads a member as `name[0].member` does, for
use as an argument or in a `let`; checks over instances use `for`.
`sum(name.member)` sums a member over the instances of one part array.
`sum(self, mass)` (shorthand `mass(self)`) rolls a parameter up over all
parts, recursively; `max(self, lead_time)` and `min(self, p)` do the
same with maximum and minimum. Roll-ups count every instance: an array
of four contributes four times. Any part lacking the parameter is an
error unless `?` is used (`sum?(self, mass)`, `mass?(self)`, `max?(self,
lead_time)`), in which case the missing parts are listed in the result's
note.

### 3.6 Tables

```
table Iso273 @iso273 {
  key thread: MetricThread
  cols fine: Length, medium: Length, coarse: Length
  M5 => 5.3mm, 5.5mm, 5.8mm
  M6 => 6.4mm, 6.6mm, 7mm
}
mount_hole = Iso273[screw.thread].medium
```

Keys are enums or quantities. `T[k]` is an exact-match row; for
quantity keys, `T.at_least(x)` is the row with the smallest key `>= x`,
`T.at_most(x)` the largest key `<= x`, and `T.nearest(x)` the closest
(ties to the larger). Every row has `.key`, so `Din7981Len.at_least(
stack).key` reads the matched length. A lookup that misses is an error
at evaluation. A literal in an `ots` that cites a table but matches none
of its rows is T001. A row with no `=>` is key-only (`table
Iso4762Len { key length: Length  3mm  4mm  5mm }`); such a table is
read for its keys (`.at_least(x).key`). The tables the standard library
ships, and their column names, are in stdlib.md.

### 3.7 Fits

`fit NAME = SPEC on PATH` inside a component, or at file scope.

- `H7/p6 on x`: both sides from ISO 286 on x's nominal.
- `p6 on x` / `H7 on x`: one-sided; the class's letter case picks the
  side it plays (lowercase shaft, uppercase hole), and x is the other
  side with its own tolerance. `p6 on wheel.hub_bore` sizes a shaft for
  a vendor bore; `H7 on motor.shaft` sizes a bore for a vendor shaft.
- `a / b`: both sides are paths with tolerances (an insert in a hole).
- An optional required range: `fit insert = hole / Insert.od [-0.4mm ..
  -0.1mm]` (negative clearance is interference).

A fit defines `NAME.hole`, `NAME.shaft`, `NAME.clearance` (toleranced)
and a computed `NAME.type` (clearance, transition, interference), and
gives each side its role. F003 when a side's own tolerance disagrees
with the class it is declared to play. Against a required range, a
clearance entirely outside it is F004 (violates); one that overlaps it
only in part is C002 on the fit (undecided), like any check.

### 3.8 Functions, enums, processes

- `fn name(a: Kind, ...) -> Kind { let x = ...; expr }`: pure, total, no
  recursion. Standard functions: `min max abs sqrt sin cos tan atan2
  round_to ceil_to floor_to floor ceil round count`, the roll-ups
  `sum max min mass` over `self` (3.5) and their `?` forms, `fmt`
  (Text from a template: `fmt("{} x {}", thread, length)` renders
  quantities by their display rules), and geometry helpers
  `rect_pattern circle_pattern distance`.
- Points: `Point2(x, y)` (and `Point3(x, y, z)`) with members `.x .y
  (.z)`; `p + q`, `p - q` and `p * s` for a dimensionless `s`.
  `rect_pattern(nx: Count, ny: Count, dx: Length, dy: Length) ->
  [Point2; nx * ny]` is centred on the origin, rows first;
  `circle_pattern(n: Count, r: Length, start: Angle) -> [Point2; n]`;
  `distance(p, q) -> Length`. `+ -` between an array of points and one
  point applies to each element (`holes + Point2(5mm, 0mm)`). Array
  literals are `[a, b, c]`. Frames are OQ-10.
- `enum Material { Al6061T6, Petg }`, `match m { Material::Petg => 2.4mm,
  _ => 1.6mm }`. A `match` must be exhaustive.
- `process` declares manufacturing compensation by role:

  ```
  process Fdm @measured:"calibration print 2026-10-01" {
    hole_comp = 0.2mm [+-0.05mm]
    rule hole(d: Length) -> Length = d + hole_comp
    rule boss(d: Length) -> Length = d - 0.1mm
  }
  model Case { #![process(Fdm)]  insert_hole = 3.4mm  #[hole] }
  ```

  The IR keeps the designed value and the drawn value; emit writes the
  drawn value. Fits, checks and every read see the designed value (the
  part as made); `x.drawn` reads the drawn value. The drawn value is
  exact (rule applied to nominals); a toleranced process parameter
  (`hole_comp [+-0.05mm]`) is one source variable added to the designed
  value of every parameter its rules touch, since the process's own
  scatter is what the made part inherits. A role is the name of a rule
  in the active process, given per member (`#[hole]` trailing); roles
  have no `#!` default, so positions are never compensated by accident.
  Under a process, a parameter whose name suggests a feature but has no
  role is P001.

### 3.9 Citations

`@name`, `@"literal"`, and an optional locator `@name:"p.2, table 1"`.
`cite name = "url-or-path"` names one; `cite name = std.iso273` cites a
standard. Built-in pseudo-citations: `@measured`, `@assumed`,
`@estimate`, each taking a locator for who and when. A component's
citation is inherited by its members unless they carry their own. Local
file citations are digest-pinned (obligations.md 3). A literal citation
(`@"ASTM A276"`) is text, not a path: it is never checked for
reachability (S002).

### 3.10 Units and kinds

`unit kip = 1000lbf`, `kind Stress = Pressure` (a distinct kind of the
same dimension), `kind HeatFlux = Power / Area` (a named kind for a
kind expression), `unit Stress MPa` (default unit for a kind). Kind
expressions are also legal wherever a kind is (`out q: Volume / Time`).

### 3.11 Attributes

`#[name(args), name(args)]` applies to the next item or member;
`#![...]` at the top of a file or component applies to everything inside
as a default (rule roles excepted, 3.8); a trailing `#[...]` after a member applies to that member.
Attributes: `emit`, `display`, `role`, `process`, `hole`/`boss`/... (rule
roles), `lifecycle`, `allow(CODE)`, and `default(Iface = Impl)`, which
sets the default for every component parameter of that interface type
in its scope (stdlib.md 4). `cadetta fmt` aligns trailing
attributes and comments into columns.

## 4. Names and resolution

- Canonical paths: ir.md 2. Several files may share a namespace.
- `use` brings names from other namespaces or packages into scope.
- Inside a component, a bare name is its own member or parameter; a
  component's arguments are in scope; `Other.member` resolves a
  component; in an assembly, `part.member` resolves an instance.
- Reference cycles are D003, found between members, not components: a
  model's argument may come from a joint that reads another member of
  the same model (`part mount: Mount(hole = screws.clearance_hole)`
  while `screws` reads `mount.thickness`). Declaration order is
  irrelevant.
- A root (a component no `part` instantiates) is evaluated only if all
  of its parameters have defaults; otherwise it is checked only through
  its instances, and if it has none that is I004.
- Case canon (N001): namespaces `lower.dotted`, types and enum variants
  `PascalCase` (digits allowed: `Iso4762M5`; an underscore between two
  digits is a decimal mark: `M2_5`), everything else
  `snake_case`. `__` is reserved for name mangling (N005).

## 5. Expressions

Precedence, high to low: postfix (`.member`, `[index]`, call, tolerance
`[+-..]`, ISO class), unary `-`, `^`, `* /`, `+ -`, `..`, `as`,
comparisons. Comparisons chain: `4mm <= x <= 6mm`. `if`/`else` and
`match` are expressions; `{ let a = ...; expr }` is a block expression.

## 6. Checks and requirements

`check NAME: expr` where expr is a comparison or `x satisfies P`, with
an optional message string. `check NAME for x in parts: expr`
quantifies (grammar in 3). The name is required: it is the check's canonical path, and
attestations and waivers bind to it.
Results are three-valued (quantities.md 5.8): C001 `violates` (error),
C002 `undecided` (warning). `require` is a check that carries a citation
to an external requirement and is tracked as an obligation
(obligations.md); it cannot be waived without an attestation.

## 7. Formatting

`cadetta fmt`: two-space indent, one statement per line, aligned `=`,
trailing attributes and comments, units and literal forms preserved.
Formatting never changes the IR digest.

## Open questions

- OQ-4: Configurations (SOLIDWORKS configurations, design tables).
  Parameterized components (3.1) cover part families; whole-assembly
  variants are still open.
- OQ-5 (closed 2026-10-13): the file extension is `.cdt`.
- OQ-6: Function-named features (`bore`, `seat`, `datum`) above raw
  parameters. Fits and process roles cover the 0.1.0 needs.
- OQ-10: Coordinate frames for positions (`frame pi at (x, y)`).
  `Point2` arithmetic covers 0.1.0; frames are deferred.
- OQ-11: Packages for shared parts libraries (versioning, fetching).
  `std` ships with the compiler (stdlib.md); third-party packages wait.
