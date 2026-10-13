# The `.cdt` language

Brace-delimited, line-terminated, declarative and total: every program
terminates and its dependency graph is known without evaluating it. The
expression language borrows Rust's flavour (expressions everywhere,
`fn`, `enum`, `match`, `#[attributes]`) without mutation, loops,
recursion, traits or closures.

## 1. Example

```
// src/lib.cdt of package acme.rover

use std::fits::iso286
use std::fasteners::{MetricThread, ScrewJoint}
use std::materials::Material

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
  holes: [Point2 on origin; 4] = rect_pattern(2, 2, 40mm, 20mm) [+-0.1mm]
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
  part bracket_screws: ScrewJoint(thread = M5, clamp = bracket[0].thickness, into = frame.material,
                                  at = bracket[0].holes)   // one joint per pattern; count(at) screws
}
```

## 2. Lexical structure

- UTF-8 source. Statements end at a newline or `;`; a newline inside
  `( ) [ ]` does not end one.
- `//` line comments; `///` doc comments attach to the next item and are
  carried into the IR and into emitted output where supported.
- Identifiers `[A-Za-z_][A-Za-z0-9_]*`. Reserved keywords: `pub priv
  package super use as cite interface ots model assembly process table enum fn let if
  else match part alt fit on solver in out deps require check for
  satisfies waive because expires was unit kind self true false datum
  place rule key cols origin and or not`.
- Quantity literals: quantities.md 2. Strings `"..."` with `\"`, `\\`.
- `@` introduces a citation (3.9). `#[...]` and `#![...]` are attributes
  (3.11). A `:` directly after a citation starts its locator string.

## 3. Items

```
file      = { use | item }                       // the module is the file's path (packages.md 2)
use       = [ "pub" [ "(" "package" ")" ] ] "use" usepath [ "as" IDENT ]
          | [ "pub" ] "use" usepath "::" ( "*" | "{" usetree { "," usetree } "}" )
usepath   = ( "package" | "self" | "super" { "::" "super" } | PKG ) { "::" IDENT }
usetree   = IDENT [ "as" IDENT ] | IDENT "::" ( "*" | "{" usetree { "," usetree } "}" )
PKG       = IDENT { "." IDENT }                  // a dependency's dotted name, or std (packages.md 4)
vis       = [ "pub" [ "(" "package" ")" ] ]      // precedes any item; default private (packages.md 3)
item      = { doc } { attr } vis ( cite | interface | component | process | table
            | enum | fn | fit | solver | require | check | waive | unit | kind )
component = ( "ots" | "model" | "assembly" ) IDENT [ params ] [ ":" iface_list ]
            [ citation ] "{" { inner_attr } { member } "}"
params    = "(" pdecl { "," pdecl } ")"
pdecl     = IDENT ":" type [ "=" expr ]              // a default makes the argument optional
member    = { doc } { attr } [ "priv" ] ( param | let | bound | part | fit | check
            | require | solver | datum | place ) [ trailing_attr ]
                                                     // "priv": usable only in its package (packages.md 3)
datum     = "datum" IDENT [ "=" expr ]               // geometry.md 2; no "=" in an interface
place     = "place" path "on" expr [ "for" IDENT "in" "0" ".." "count" "(" path ")" ]
                                                     // geometry.md 4; the range form indexes PART[i]
type      = IDENT | "Datum" | ( "Point2" | "Point3" ) "on" path
            | "[" type ";" expr "]" | "Table" "<" type ">"   // Vec2, Vec3, Point2, Point3: geometry.md 3
let       = "let" IDENT [ ":" type ] "=" expr        // analysis value (quantities.md 5.3)
param     = IDENT [ ":" type ] "=" expr [ citation ] { "was" path }
bound     = IDENT ( "<=" | ">=" ) expr | IDENT "in" expr ".." expr | path "==" path
part      = "part" IDENT [ "[" expr "]" ] ":" type [ "=" ctor [ "alt" "[" ctor { "," ctor } "]" ] ]
ctor      = path [ "(" args ")" ]
check     = "check" IDENT [ "for" IDENT "in" expr ] ":" expr [ STRING ]
process   = "process" IDENT [ citation ] "{" { param | rule } "}"
rule      = "rule" IDENT "(" pdecl ")" "->" type "=" expr
table     = "table" IDENT [ citation ] "{" "key" pdecl [ "cols" pdecl { "," pdecl } ]
            { expr [ "=>" expr { "," expr } ] } "}"   // no "=>": a key-only row
cite      = "cite" IDENT "=" ( STRING | path )   // path: a standard, `std::iso273`
interface = "interface" IDENT [ ":" iface_list ] "{" { doc | bound | IDENT [ "?" ] ":" type | datum } "}"
                                                     // ":" extends; "?" marks an optional member
enum      = "enum" IDENT "{" IDENT { "," IDENT } [ "," ] "}"
fn        = "fn" IDENT "(" [ pdecl { "," pdecl } ] ")" "->" type
            "{" { let } expr "}"
fit       = "fit" IDENT "=" fitspec "on" path [ "[" expr ".." expr "]" ]
fitspec   = CLASS [ "/" CLASS ] | path "/" path    // CLASS: H7, p6; 3.7
require   = "require" IDENT ":" expr [ STRING ] [ citation ]
solver    = "solver" IDENT citation "{" { "in" bind { "," bind }
            | "out" IDENT ":" type | "deps" STRING } "}"
bind      = IDENT "=" expr
waive     = "waive" CODE "on" GLOB [ "for" "alt" SLOT "=" ALT ]
            "{" "because" STRING [ "expires" DATE ] "}"
unit      = "unit" IDENT "=" expr | "unit" IDENT IDENT   // definition | default unit of a kind
kind      = "kind" IDENT "=" kindexpr             // kindexpr: quantities.md 2
citation  = "@" ( IDENT | STRING ) [ ":" STRING ]
```

`GLOB` is the path-glob grammar of obligations.md 4, `SLOT` a canonical
slot path and `ALT` a qualified alternate (3.2); `CODE` is a diagnostic
code (lints.md).

### 3.1 Component classes

| Class | Meaning | May contain |
|---|---|---|
| `ots` | Off-the-shelf: facts about a part someone else makes. | Literals and expressions over its own parameters (a drawing states pitch and positions). Every value is cited (S001). Commercial data (`unit_cost`, `lead_time`, `moq`, `supplier`) is ordinary parameters. |
| `model` | A part we design. | Anything. |
| `assembly` | A composition of `part` instances. | Parameters, `let`s, parts, `place`s, fits, checks, requires, solvers. |

All three classes may declare `datum`s; in an `ots` a datum offset is a
fact about the part and is cited (S001). `place` is for assemblies only.
`origin` is predefined in every component; a member named `origin` is an
error.

Components may take parameters (`ots Iso4762(thread: MetricThread,
length: Length)`, `model Axle(wheel: Caster)`), optionally with defaults.
Arguments are given at the `part` site. A parameterized `ots` is a part
family; its values are typically table lookups (3.6). A lookup that
misses is E007.

`Type.member` reads a member of a parameterized component without an
instance when that member does not depend on any parameter
(`TrScrew.cut_increment`), so an argument can be computed from the
family itself: `part screw: TrScrew(length = ceil_to(min_len,
TrScrew.cut_increment))`. Reading a member that does depend on a
parameter this way is E011 naming the parameter.

Parameterized components exist only as instances: a parameter-dependent
member has a path only under a `part` (`Rover/axle[0].end_diameter`),
and reads, bindings and obligations name that path (ir.md 2). Reading
such a member by the type name is E011.

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
`: Caster`; a missing member is I001 (error) unless the interface
marks it optional (`unit_cost?: Money`), in which case `?` roll-ups list
it as missing; a violated bound is C001 against that implementer.
A member present with a different kind is also I001; an undecided bound is
C002 against the implementer. I001 is fixed and cannot be waived.

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
  is reported once, as I002 on the alternate, naming each check, fit,
  bound or require it violates (`for alt Rover/wheel=Colson2Series`);
  C001 and C002 are the primary's verdicts. I002 is a warning by default,
  so an approved alternate that stops working does not break the build
  of a design that still works; a shop that wants it to raises I002 to
  an error (lints.md 1). An evaluation error under a variant (E007
  lookup miss, a failed `as`) also makes that alternate non-viable, with
  the error as the reason; it never fails the run.
- **Viable** means no check, fit, bound or require violates under the
  variant. Undecided results leave it viable with I003. A non-viable
  alternate stays listed (I002); a waiver may be scoped to one
  alternate (`waive C002 on acme.rover::Rover.load_ok for alt
  Rover/wheel=Colson2Series`), but a `require` is never suppressed (6),
  so a violated one makes an alternate non-viable.
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
  when the status was last verified and never raises L001/L002. A
  `YYYY-MM` date covers its whole month: it has passed only once
  `--as-of` is in a later month; `YYYY-MM-DD` has passed the day after.
  An `eol` or `obsolete` primary is not L001 when some approved
  alternate is viable and is not itself `eol` or `obsolete`; the viable
  alternates are listed as its replacement candidates.

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
  `Bool`, points typed by a datum (`Point2 on top`, `Point3 on origin`,
  and `Vec2`, `Vec3`; geometry.md 3), fixed arrays `[T; N]`, and `Table<K>`, a
  reference to any table keyed by `K` whose columns are read as usual
  (`policy.lengths.at_least(x).key`). Columns a reader uses must exist
  in the table passed, with the kinds the reader expects; both are
  checked at the use site, where a missing column or a kind mismatch is
  E008 naming the table.

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
a Count expression. It may read parameters, `count()` and members of
other parts; it is evaluated first, together with exactly the members
it reads, so the part graph is known before the rest of the design is
evaluated. D003 covers a size that depends on the parts it sizes. Instances are distinct: each toleranced parameter of
each instance is an independent source variable (quantities.md 5.4).
`count(name)` is a Count; `name[i]` an instance; `for x in name`
iterates in checks. Every instance of an array has the same arguments,
so `name.member` (no index) reads a member as `name[0].member` does, for
use as an argument or in a `let`; checks over instances use `for`.
`sum(name.member)` sums a member over the instances of one part array.
`sum(self, mass)` (shorthand `mass(self)`) rolls a parameter up over all
parts, recursively; `max(self, lead_time)` and `min(self, p)` do the
same with maximum and minimum. Roll-ups count every instance: an array
of four contributes four times. The roll-up
names `unit_cost lead_time moq supplier mass` are exactly the members
of `std::commercial::Sourced`; swap commercial deltas (obligations.md
5.1) and `compare` columns use the same set. A wrong kind on one is an
ordinary I001 or Q003. Any part lacking the parameter is an
error unless `?` is used (`sum?(self, mass)`, `mass?(self)`, `max?(self,
lead_time)`), in which case the missing parts are listed in the result's
note.

`N` must be an exact, dimensionless, non-negative whole number. An
unindexed read of an array whose `N` is 0 is an error, as is an index
outside `0..N`. `max` and `min` compare nominals and return the whole
toleranced value of the winner; a roll-up over values of different
dimensions is an error, and so is one with no contributing part.

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
stack).key` reads the matched length. A lookup that misses is E007 at
evaluation. A literal in an `ots` that cites a table but matches none
of its rows is T001. A row with no `=>` is key-only (`table
Iso4762Len { key length: Length  3mm  4mm  5mm }`); such a table is
read for its keys (`.at_least(x).key`); reading a column of a
key-only table, or using `at_least`, `at_most` or `nearest` on a table
whose keys are not quantities of the argument's dimension, is E008.
The tables the standard library
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

Tabulated classes (cadetta-fits, 0.1): nominal sizes over 0 up to and
including 500 mm, size ranges are (lo, hi]; IT1..IT18; shaft letters
`c d e f g h js j k m n p` and hole letters `C D E F G H JS`; `j` only
for grades 5 to 7. Any other class is F001 (not yet tabulated), a size
outside (0, 500] mm is F002. A class letter is wholly upper or wholly
lower case; mixed case is F001. `js`/`JS` is exactly +-IT/2 (no
rounding of odd IT widths). Values are exact integer nanometres since
IT1..IT4 are tabulated to 0.1 um.

A fit defines `NAME.hole`, `NAME.shaft`, `NAME.clearance` (toleranced)
and a computed `NAME.type` (clearance, transition, interference), and
gives each side its role. F003 when a side's own tolerance disagrees
with the class it is declared to play. Against a required range, a
clearance entirely outside it is F004 (violates); one that overlaps it
only in part is C002 on the fit (undecided), like any check.

Decisions (cadetta-sema `fits`): the clearance is `hole - shaft`
over independent variables, so its bounds are `hole.min - shaft.max`
and `hole.max - shaft.min`; negative is interference. `NAME.type` is
`clearance` when every clearance is >= 0, `interference` when every
clearance is <= 0 (and not clearance), else `transition`. A required
range is two comparisons, `clearance >= lo` and `clearance <= hi`; either
violating is F004, both holding is fine, anything else is C002. In `a /
b` the first path is the hole and the second the shaft. For `H7/p6 on
x` the fitted parameter x may be untoleranced or carry exactly one of
the two classes' deviations; any other tolerance on it is F003. A
nominal that is not a whole number of nanometres has no table row (F002).

### 3.8 Functions, enums, processes

- `fn name(a: Kind, ...) -> Kind { let x = ...; expr }`: pure, total, no
  recursion. Standard functions: `min max abs sqrt sin cos tan atan2
  round_to ceil_to floor_to floor ceil round count`, the roll-ups
  `sum max min mass` over `self` (3.5) and their `?` forms, `fmt`
  (Text from a template: `fmt("{} x {}", thread, length)` renders
  quantities by their display rules), and geometry helpers
  `rect_pattern circle_pattern distance`.
- Points: `Point2(x, y)` (and `Point3(x, y, z)`) with members `.x .y
  (.z)`, typed by a datum (geometry.md 3). `Vec2(x, y)` and `Vec3`
  construct vectors: `p - q` is a vector, `p + v` a point, `v * s`
  scales a vector by a dimensionless `s`. `Point3(...) on D` is the
  typing form of a literal (both point types); `p in D` applied to a
  typed point is re-expression only, and `in` maps over arrays.
  `D.at(point)` is datum `D` offset to the point's x and y (z).
  `rect_pattern(nx: Count, ny: Count, dx: Length, dy: Length, rotation:
  Angle = 0deg) -> [Point2; nx * ny]` is centred on the origin; dx and
  dy are pitches (centre to centre); points are ordered row by row, x
  fastest, from (-,-) to (+,+). `circle_pattern(n: Count, r: Length,
  start: Angle) -> [Point2; n]`; `distance(p, q) -> Length`. Axis
  builtins: `radial(p: Point3) -> Length` (distance from the z axis of
  p's datum, square-aware interval, quantities.md 5.4), `coaxial(a:
  Datum, b: Datum, allowance, over: Length)` (a radius zone; the
  diameter form is `2 * radial`), `parallel(a, b, allowance, over)` and
  `angle_between(a, b) -> Angle`. `+ -` between an array of points and
  one vector shifts every element (`holes + Vec2(5mm, 0mm)`); array +
  point is G001. Generators are values with members that bind
  (`holes.pitch_x`, `.pitch_y`, `.radius`, `.start`, `.rotation`;
  ir.md 2). Array
  literals are `[a, b, c]`. `aligned(a, b, allowance)` compares two
  patterns (geometry.md 5).
- `enum Material { Al6061T6, Petg }`, `match m { Material::Petg => 2.4mm,
  _ => 1.6mm }`. A `match` must be exhaustive (E009).
- `process` declares manufacturing compensation by role:

  ```
  process Fdm @measured:"calibration print 2026-10-01" {
    hole_comp = 0.2mm [+-0.05mm]
    rule hole(d: Length) -> Length = d + hole_comp
    rule boss(d: Length) -> Length = d - 0.1mm
  }
  model Case { #![process(Fdm)]  insert_hole = 3.4mm  #[hole] }
  ```

  The IR's `nominal` is always the designed value, and `drawn?` holds
  the drawn value when a rule applied; emit writes the drawn value of
  the emit point (quantities.md 5.6). Fits, checks and every read see
  the designed value (the part as made); `x.drawn` reads the drawn
  value. The drawn value is
  exact (rule applied to nominals); a toleranced process parameter
  (`hole_comp [+-0.05mm]`) is one source variable added to the designed
  value of every parameter its rules touch, since the process's own
  scatter is what the made part inherits. A role is the name of a rule
  in the active process, given per member (`#[hole]` trailing); roles
  have no `#!` default, so positions are never compensated by accident.
  Under a process, a parameter whose name suggests a feature but has no
  role is P001.
  Decisions (cadetta-sema `process.rs`): a process is found by name in
  the component's package, then in any package; a component's
  `#![process(P)]` wins over the file's; a role is a bare attribute
  naming a rule of the active process, on the member (leading or
  trailing), never inherited from a scope. A rule body may read its
  argument, the process's parameters and literals, and use `+ - * /`,
  parentheses and negation; it must keep the argument's kind. The
  designed value keeps the member's nominal and gains, for each process
  parameter a rule reads, that parameter's deviation about its centre.
  A name suggests a feature when a `_`-separated word is one of `hole
  bore boss pin slot post pocket shaft stud standoff peg cutout rib`.

Arrays and points (decisions, cadetta-sema `array.rs`, `point.rs`): an
array literal holds elements of one kind (one dimension, one point
arity, one enum) or it is an error; a declared `[T; N]` checks the
length against N (a whole number) and a `Point2`/`Point3` element type
checks the arity. Until datum typing lands (geometry epic) points and
vectors share one value: `Point2(x, y)`, `Vec2(x, y)` and the 3D forms
take Length components, `p + q`, `p - q` and `p * s` (or `p / s`, `s *
p`; `s` dimensionless) each give a point, `.x .y .z` read a component,
and an array combined with a point (either order, `+ -`) applies the
point to every element. Array with array is an error. Pattern points
are exact when the angle is a special one (`circle_pattern(4, 10mm,
0deg)`) and carry an inexact interval otherwise (Q006); `distance` is
exact when the sum of squares is a perfect square. A component that
carries a tolerance keeps it, so `rect_pattern(2, 2, 30mm [+-0.05mm],
40mm)` shifts the points with the pitch.

Evaluation of these (decisions): function bodies and `let` values are
analysis values, evaluated over the full tolerance of their operands;
only a parameter stores a design tolerance, and a pure reference is an
alias (quantities.md 5.3). `match` arms are chosen on the scrutinee's
variant and a missing variant without `_` is E009.

### 3.9 Citations

`@name`, `@"literal"`, and an optional locator `@name:"p.2, table 1"`.
`cite name = "url-or-path"` names one (only `file` citations are
drift-detected, S004); `cite name = std::iso273` cites a
standard. Built-in pseudo-citations: `@measured`, `@assumed`,
`@estimate`, each taking a locator for who and when. A component's
citation is inherited by its members unless they carry their own. Local
file citations are digest-pinned (obligations.md 3). A literal citation
(`@"ASTM A276"`) is text, not a path: it is never checked for
reachability (S002).

Citation kinds in the IR: a `cite` string starting `http://` or `https://`
is `url`, any other string is `file` (a path relative to the package
directory, digest recorded when the file exists), a `std::...` path or a
literal is `standard`, and `@measured`, `@assumed`, `@estimate` are their
own kinds with the locator carrying who and when. A member with no
citation of its own inherits the component's; a member citation replaces
it, it is never merged.

### 3.10 Units and kinds

`unit kip = 1000lbf`, `kind Stress = Pressure` (a distinct kind of the
same dimension), `kind HeatFlux = Power / Area` (a named kind for a
kind expression), `unit Stress MPa` (default unit for a kind). Kind
expressions are also legal wherever a kind is (`out q: Volume / Time`).

### 3.11 Attributes

`#[name(args), name(args)]` applies to the next item or member;
`#![...]` at the top of a file or component applies to everything inside
as a default (rule roles excepted, 3.8); a trailing `#[...]` after a
member applies to that member.

| Attribute | Meaning |
|---|---|
| `emit(point)`, `display(unit)` | Presentational: emitted point (quantities.md 5.6) and display unit. |
| `role(hole\|shaft)`, `process(P)`, `hole`/`boss`/... | Semantic: tolerance role, process, rule roles. |
| `lifecycle(status, "date")` | Feeds L001/L002 (3.2). |
| `native(adapter, "locator")` | Binds the member to a CAD locator inline, instead of in `bindings/<id>.toml` (adapters.md 3). |
| `allow(CODE, "because")` | A waiver for one member: the reason is required, it has no expiry, W004 checks it suppresses something, and W002 does not apply to it (lints.md 2). |
| `default(Iface = Impl)` | Sets the default implementer for every component parameter of that interface type in scope. |

Default precedence, highest first: the enclosing component's
`#![default]`, the file's `#![default]`, the project's `[defaults]`
(config-cli.md), then the standard library's (stdlib.md 4).
Precedence among attributes of the same key, lowest first: file `#![]`,
component `#![]`, the member's leading `#[]`, the member's trailing `#[]`.
A bare attribute (a rule role such as `#![hole]`) and `role(...)` never
inherit from a scope. `default(Iface = Impl)` and `allow(CODE, "why")` are
keyed per interface or code, so several coexist. In the IR the attribute
value is its arguments joined by `, `, string literals unquoted.
`cadetta fmt` aligns trailing attributes and comments into columns.

## 4. Names and resolution

- Canonical paths: ir.md 2. A file's module is its path under the
  package's `src`; packages, modules, visibility, `use` and `pub use`
  are defined in packages.md, and its section 4 is the single
  definition of `use` and name resolution. The project model is in
  config-cli.md "Project model".
- `use` brings names from other modules and packages into scope
  (`use package::frame::Frame`, `use std::fits::*`, `use self::x as y`).
  A fully qualified path (`std::fasteners::ScrewJoint`) is legal
  anywhere without a `use`.
- A `use` imports every kind of item (components, interfaces, tables,
  enums, functions, processes, citations, units and kinds). Enum
  variants come through their enum (`MetricThread::M5`, or bare where
  the type is known, 3.1).
- Resolution order for a bare name is packages.md 4: the component's
  own members and arguments, items of the current module, explicit
  `use` names, glob names, then the prelude. Two globs or uses that
  bring the same name are E004 only where the name is used.
- The prelude needs no `use`: the units and kinds of quantities.md, the
  built-in types (`Bool Text Count Ratio Point2 Point3 Vec2 Vec3 Datum
  Table`), the built-in functions (3.8), `origin` and `self`. Nothing
  from `std` is in the prelude.
- Items are private by default; `pub` and `pub(package)` widen them
  and `pub use` re-exports (packages.md 3, 4). A move between modules
  or packages is a rename: `was` takes the full canonical old path
  (`was acme.old::Bracket.thickness`, `was acme.rover::frame::Bracket`).
- Resolution errors are fixed-severity E codes: E002 unresolved name,
  E003 duplicate definition of one Type in a module (both spans
  shown), E004 two `use`s or globs bringing the same local name where it is
  used (ambiguous import), E016 private item, E017 private-in-public
  (packages.md 7), E006 argument mismatch at a `part` site. A declaration cycle
  is D003, not an import cycle: files may import each other.
- Inside a component, a bare name is its own member or parameter; a
  component's arguments are in scope; `Other.member` resolves a
  component; in an assembly, `part.member` resolves an instance.
- Reference resolution (decided in the resolver, `cadetta-sema`
  `resolve.rs`): a reference is a path plus `.field` and `[index]`
  steps. If it contains `::`, the part before the first `::` is a
  package (`acme.rover`, `std`), `package`, `self`, `super`, or a local
  name (an enum or module alias); the `::` names then descend modules to
  an item (a module's parent may have no file), and an enum item takes
  one variant. Otherwise the first name is looked up as a local binding,
  then the component's own argument or member or `part`, then module
  items, explicit `use`, globs and the prelude. A component argument is
  not a member: `Type.argument` is E011. `Type.member` is checked for
  parameter dependence through the member's definition and the members
  it reads (including `part` arguments); the first parameter in
  declaration order is named in E011. A bare enum variant is accepted
  only as an argument whose declared parameter type is that enum, and
  only when the name is not otherwise in scope. An index in a reference
  must be an integer literal, because a canonical path carries only
  integers (ir.md 2); other indexes are for the evaluator. Visibility
  (E016, E017) is checked by the lint family, not by resolution.
- Reference cycles are D003, found between members, not components: a
  model's argument may come from a joint that reads another member of
  the same model (`part mount: Mount(hole = screws.clearance_hole)`
  while `screws` reads `mount.thickness`). Declaration order is
  irrelevant.
  The graph's nodes are the members of instances (parameters, `let`s,
  checks, `part` slots and component arguments); it is built from the
  syntax and name resolution alone, before any evaluation (`cadetta-sema`
  `deps.rs`). Reading a member of an array `part` also depends on the
  part's slot, because instances exist only once the count is known; a
  roll-up (`sum(self, mass)`, `mass(self)`) depends on the slot and the
  named member of every part, never on the member that contains it.
- A `was` path is read from its text, since the old item may no longer
  exist. A reference that fails to resolve but names a `was` path
  resolves to the parameter's current path and raises N004.
- A root (a component no `part` instantiates) is evaluated only if all
  of its parameters have defaults; otherwise it is checked only through
  its instances, and if it has none that is I004, which applies only to
  roots of the package being checked; a private unused parameterized
  component in a library is U002 (packages.md 3), so a shared library
  is not noisy.
- Case canon (N001): package names `lower.dotted`, modules `snake_case`, types and enum variants
  `PascalCase` (digits allowed: `Iso4762M5`; an underscore between two
  digits is a decimal mark: `M2_5`), everything else
  `snake_case`. `__` is reserved for name mangling (N005).

## 5. Expressions

Precedence, high to low: postfix (`.member`, `[index]`, call, tolerance
`[+-..]`, ISO class), unary `-`, `^`, `* /`, `+ -`, `..`, `in`,
`as`, comparisons, `satisfies`, `not`, `and`, `or`. A member beginning
`IDENT in` is a bound (3.4) only when the right side is a range.
Comparisons chain: `4mm <= x <= 6mm`. `if`/`else` and
`match` are expressions; `{ let a = ...; expr }` is a block expression.

## 5a. Parser decisions

Rules the grammar left open, fixed by the reference parser:

- `part name: Type(args)` without `= ...` is allowed (a part whose type
  is constructed directly, as `part axle[2]: Axle(wheel = ...)`).
- A postfix `[` followed by `+` or `-` opens a tolerance bracket
  (`12mm [+0.018mm -0]`, `[+-0.1mm]`); any other `[` is an index. Each
  tolerance term is a sign (`+`, `-`, or `+-` with no space) and a
  product-level expression.
- An ISO class (`H8`, `p6`, `js6`: one or two letters then one or two
  digits) directly after an expression on the same line is a postfix
  class; a `fit` spec is a lone class, `class / class`, or two paths.
- Argument lists are `[name =] expr` separated by commas; interface
  lists after `:` are comma-separated paths.
- A trailing `#[...]` attribute must be on the same line as its member.
- `Table<K>` is a type; a space is needed before a following `=`
  because `>=` lexes as one token.
- A keyword is accepted as a member name after `.` (`T.at_least(x).key`,
  `row.in`); elsewhere see the next rule.
- Expression precedence is pinned by the conformance tests in
  `crates/cadetta-syntax/src/expr.rs`, one per rule of section 5.
- Keywords used as names are errors (E001). `origin` is one: a member,
  datum or interface field named `origin` is E001 with a message pointing
  at geometry.md 2.
- `datum NAME = expr` needs its value in a component and forbids it in an
  interface. `place TARGET on expr [for i in lo..hi]` takes a postfix
  expression as TARGET (`rail.origin`, `feet[i].top`); the `for` range is
  `lo..hi` with any additive bounds, and the semantic check that it is
  `0..count(PART)` is sema's. `p in D` is the ordinary `in` expression
  (section 5); at member level `IDENT in` is a bound, so it must be
  followed by a range (`w in D` alone is E001).
- Kind expressions (`Volume / Time`, `Pressure / VolumeFlow^2`) are a
  left-to-right product or quotient of kind paths, each with an optional
  integer exponent (`^2`, `^-1`); no parentheses and no rational
  exponents. They are accepted wherever a type is (`out q: Volume /
  Time`, parameters, `kind X = ...`). A lone kind stays a plain path.
- A `table` needs exactly one `key` line (else E001); `cols` is an
  optional comma list. A row with `=>` ends its line; key-only rows
  (no `=>`) may share a line, as in `key length: Length  3mm  4mm`.
- A `process` body holds parameters and `rule NAME(p: Kind) -> Kind =
  expr` (exactly one parameter). An `enum` needs at least one variant;
  commas and newlines separate variants, a trailing comma is allowed.
- A `solver` (item or component member) requires a citation naming its
  script; its body holds `in a = expr, ...`, `out NAME: kind` and
  `deps STRING`.
- `waive CODE on GLOB [for alt SLOT = ALT] { because STRING [expires
  DATE] }`: the glob, slot and alternate are kept as raw text made of
  adjacent tokens (no whitespace; `*`, `**`, `[*]`, `::`, `.`, `/`
  allowed). DATE is strictly `YYYY-MM-DD`, else E001.
- `unit NAME = expr` defines a unit; `unit KIND UNIT` sets a kind's
  default display unit.
- Source locations: lines and columns are 1-based; a column counts
  Unicode characters, not bytes; the end position is exclusive and
  prints as `file:line:col-line:col`. A byte offset inside a character
  snaps to that character start; one past the end clamps to the end. An
  item span includes its trailing newline.

## 6. Checks and requirements

`check NAME: expr` where expr is a comparison or `x satisfies P`, with
an optional message string. `check NAME for x in parts: expr`
quantifies (grammar in 3). The name is required: it is the check's
canonical path, and attestations and waivers bind to it.
Results are three-valued (quantities.md 5.8): C001 `violates` (error),
C002 `undecided` (warning). `require` is a check that carries a citation
to an external requirement and is tracked as an obligation
(obligations.md).

A `require` is never suppressed. Its verdict is evaluated like any
check. An attestation is sign-off on a verdict, never a waiver: it
discharges the sign-off obligation of a `holds` require (O003) and
cannot change a `violated` one, which stays C001. The state `waived`
applies only to diagnostics suppressed by `waive` or `allow`. The single
rule is in obligations.md 4.

Decisions (cadetta-sema `checks`): a check condition is a comparison
chain, `x satisfies P`, or `and`, `or` and `not` over those, combined
with Kleene logic over holds, violates and undecided; a third kind,
`==` on toleranced values, is C003 (warning). A bound is looked up
syntactically, either bare (`P`, in the enclosing component) or as
`Item.P`, because bounds are not value members; when a parameter and a
bound share a name, `P.min`/`P.max` read the parameter and `satisfies
P` reads the bound. A bound's limits are the nominals of its
expressions. A quantifier ranges over an array literal, one result per
instance; ranging over a `part` array waits for the parts work. A
`require` carries its own citation, else its component's, and is
marked as an obligation (obligations.md).

## 7. Formatting

`cadetta fmt`: two-space indent, one statement per line, aligned `=`,
trailing attributes and comments, units and literal forms preserved.
Formatting never changes the IR digest.

Formatter decisions (cadetta-syntax `format`):

- A file with any syntax error is refused, never formatted away.
  `datum` and `place` members (and `datum` in interfaces) print on one
  line (`datum top = origin.offset(z = height)`, `place a.b on c for i in
  0..count(P)`) and take no part in `=` alignment.
- `enum`, `fn`, `waive`, `unit` and `kind` items print on one line
  (`enum G { A, B }`, `waive C on GLOB { because "x" }`); `unit` and
  `kind` definitions align their `=` with neighbours. `process`,
  `table` and `solver` (item or member) print as blocks: process
  parameters like component parameters; a table prints `key`, then
  `cols`, then one row per line with `=>` aligned in a group (key-only
  rows that shared a line are split); solver bodies print `in`, `out`,
  `deps` one per line. Kind expressions print as `A / B^2`, and a
  member `solver` keeps its trailing attributes after the closing `}`.
- LF newlines, one final newline, no line wrapping: multi-line
  expressions collapse to one line (`{ let a = 1; a + b }`,
  `match s { A => 1, _ => 2 }`). Blank lines survive, at most one in a row,
  none after `{`.
- Consecutive single-line statements form an alignment group (broken by a
  blank line, own-line comment, doc, leading attribute or a block). In a
  group `=` is aligned, then trailing attributes (`#[a, b]`, merged) and
  trailing `//` comments are aligned into columns, two spaces apart.
- Own-line comments stay before the statement they precede. A comment
  inside a statement is hoisted above it. A same-line trailing comment
  stays trailing.
- Numbers, quantity literals and citations are copied from the source
  text; strings are re-quoted with the two escapes.
- The output is re-parsed and its tree (spans aside) must equal the
  input's, otherwise the formatter returns an error and emits nothing.
  This is the guarantee behind "never changes the IR digest".
- `is_formatted` backs `cadetta fmt --check`: it reports and changes nothing.

## Open questions

- OQ-4 (deferred past 0.1.0): Configurations (SOLIDWORKS
  configurations, design tables). Parameterized components (3.1) cover
  part families; whole-assembly variants wait. A locator binds the
  active configuration only (adapters.md "Import").
- OQ-5 (closed 2026-10-13, language.md 2): the file extension is `.cdt`.
- OQ-6: Function-named features (`bore`, `seat`, `datum`) above raw
  parameters. Fits and process roles cover the 0.1.0 needs.
- OQ-10 (closed 2026-10-13, geometry.md): positions are typed by
  datums, and parts are placed in a tree, which avoids constraint
  solving.
- OQ-11 (closed 2026-10-13, packages.md): packages, modules and
  visibility follow Rust's model; `std` ships with the compiler
  (stdlib.md). A registry is OQ-19; member privacy is `priv`
  (OQ-20, closed).
