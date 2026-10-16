# The standard library

`std` ships with the compiler, versioned with it, and is written in
ordinary `.cdt`: every table is a `table`, every part family a
parameterized `ots`, every helper a `fn` or a parameterized `assembly`.
Nothing in `std` needs a language feature user code cannot use; if it
did, that feature would be missing from the language. Every value in
`std` is cited to its standard or handbook (S001 applies to `std` too). The `std_s001` test target (`crates/cadetta-cli/gates/std_s001.rs`) analyses a
copy of `std/` renamed `stdgate` (`std` is reserved) as a project and fails on any S001, so a new uncited value cannot land.
`std` exports one `cite` per standard at its root (`std::iso273`,
`std::iso286`, `std::din472`, ...), each a typed `standard` or `book`
citation (language.md 3.9) with `organization`, `number`, `title` and
`date` or `edition`, so `cadetta cite list` prints them in full. User
code cites a standard as `@std::din472`, or `use std::din472` and then
`@din472`; it never re-declares one. Library intermediates are `priv` members
(packages.md 3): `ScrewJoint`'s public members are the ones listed in
section 3, and its other values are `priv`.

Why a library and not syntax: a screw is shop knowledge (clearance
classes, tap drills, engagement rules, preferred lengths), and shops
disagree. Data can be cited, overridden and diffed; syntax cannot.

## 1. Modules

| Module | Contents | 0.1.0 |
|---|---|---|
| `std::fits` | ISO 286 classes (`iso286`) and the fit machinery behind `fit` | yes |
| `std::materials` | `Material` enum and the `Materials` table: density, yield, tap class, engagement factor | yes, a short list |
| `std::fasteners` | Metric threads, ISO 4762 socket head cap screws, ISO 4032 nuts, ISO 273 clearance holes, tap drills, preferred lengths, `ScrewJoint` | yes, metric coarse |
| `std::retaining` | DIN 471/472 retaining rings and grooves | yes |
| `std::commercial` | The `Sourced` interface: unit cost, lead time, MOQ, supplier, mass (section 6) | yes |
| `std::bearings` | Deep-groove ball bearing envelopes (ISO 15) | later |
| `std::fasteners::inch` | UNC/UNF, ASME B18.3, inch clearance and tap drills | 0.2.0 |
| `std::inserts` | Heat-set and wire-thread inserts | 0.2.0 (pending) |
| `std::fasteners` thread-forming | ISO 7049/7085 thread-forming screws and pilot holes | later |

Paths are `std::module::Item`, and `std` is an implicit dependency
(packages.md 1). Everything it exports is `pub` (packages.md 3).

## 2. Tables and their columns

Column names are part of `std`'s interface: renaming one is a breaking
change and is recorded with `was`.

| Table | Key | Columns |
|---|---|---|
| `std::fits::ItGrade` | `hi` (upper bound of a size range, (lo, hi]) | `it1 ... it18` (tolerance width) |
| `std::fits::ShaftUpper` | `hi` | `d e f g h` (upper deviation es) |
| `std::fits::ShaftUpperC` | `hi` (25 finer ranges) | `c` (upper deviation es) |
| `std::fits::ShaftLower` | `hi` | `j_5_6 j_7 k_4_7 m n p` (lower deviation ei) |
| `std::fasteners::Iso273` | `MetricThread` | `fine medium coarse` (clearance hole diameters) |
| `std::fasteners::Thread` | `MetricThread` | `major pitch minor_ext minor_int` |
| `std::fasteners::TapDrill` | `MetricThread` | `cut_75 cut_65 form` (cutting tap at about 75% and 65% thread, forming tap) |
| `std::fasteners::Iso4762Head` | `MetricThread` | `head_dia head_height socket counterbore_dia counterbore_depth` |
| `std::fasteners::Iso7380Head` | `MetricThread` (M3 to M16) | as `Iso4762Head`; the counterbore columns are derived from the head |
| `std::fasteners::Iso10642Head` | `MetricThread` (M3 to M20) | as `Iso4762Head`; the pocket is the countersink (`head_dia` by `head_height`) |
| `std::fasteners::Iso4762Len` | `Length` | (preferred lengths; key only) |
| `std::fasteners::Iso4032` | `MetricThread` | `width_across_flats height` |
| `std::materials::Materials` | `Material` | `density yield_strength tap_class engagement_factor` |
| `std::retaining::Din471` / `Din472` | `Length` (shaft or bore) | `groove_dia groove_width ring_thickness edge_margin` (each with its published tolerance) |

The `std::fits` tables are cited to ISO 286-1:2010 and 286-2:2010
(`std::fits::iso286`). Decisions: sizes are keyed by the upper bound of
their `(lo, hi]` range, so `T.at_least(size).key` selects the range;
hole classes are not stored, they mirror the shaft of the same letter
(EI = -es for C to H) and `JS`/`js` is exactly +-IT/2; `k` outside
IT4 to IT7 is 0 and `j` exists for IT5 to IT7 only (language.md 3.7).

`Material` variants in 0.1.0: `Al6061T6 Al7075T6 Al5052H32 Ss304 Ss316
Steel1018 Steel4140 CastIron Brass360 Petg Pla Abs Nylon6 Pom`. Each row
is cited to a materials handbook. `MetricThread` variants: `M1_6 M2
M2_5 M3 M4 M5 M6 M8 M10 M12 M16 M20` (an underscore between digits is
the decimal mark; `M2_5` displays as `M2.5`). `TapClass`: `Soft Hard`.
`TapColumn`: `Cut75 Cut65 Form`, naming the `TapDrill` columns
`cut_75 cut_65 form`.

`Materials` units and decisions: `density` in g/cm^3, `yield_strength`
in MPa, both handbook nominal values for the stock temper the variant
names (design-stage, not procurement minima); `engagement_factor` is
steel and stainless 1.0, brass and cast iron 1.5, aluminium and polymers
2.0; `tap_class` is `Hard` for steels and stainless, `Soft` otherwise.
Engagement and tap class are shop policy, not published constants.
`CastIron` (ASTM A48 class 30) has no yield point, so its
`yield_strength` holds the minimum tensile strength as a stand-in.

`Din471`/`Din472` decisions: keyed by nominal shaft/bore d1 in mm (Din471
3 to 75, Din472 8 to 75, preferred sizes only); `groove_dia` carries `h10`
(shaft) or `H11` (bore) and `groove_width` carries `H13`; `ring_thickness`
is the nominal s (its deviation is written on the `ots`, golden 02);
`edge_margin` is the minimum n and untoleranced. Figures were entered
from recall and must be verified by the std citation audit.

`std` is version 0.2.0 (`std/cadetta.toml`). `std::fasteners::inch`
(`std/src/fasteners/inch.cdt`) adds the unified inch threads: the
`InchThread` enum (UNC and UNF, #4 to 1/2 in; `No10Unc24` is #10-24,
`F1x4Unf28` is 1/4-28) and the tables `InchThreadData` (`major pitch
minor_ext minor_int`, ASME B1.1), `InchTapDrill` (`cut_75`),
`InchClearance` (`close free`), `InchSocketHead` (`head_dia head_height
socket`, ASME B18.3) and `InchLen`, all in inches and each cited
(`asme_b1_1`, `asme_b18_3`, `inch_tapdrill`, `inch_clearance`). Minor
diameters use the ISO 724 profile constants. Figures were entered from
recall and the citation audit must verify them. `std::inserts` (heat-set
and wire-thread) is not yet present: heat-set inserts are vendor parts
with no standard to cite.

`std::fasteners` tables (`std/src/fasteners/tables.cdt`) decisions:
`MetricThread` is declared there with the 12 coarse sizes; `Thread`
holds `minor_ext` d3 = d - 1.22687 P and `minor_int` D1 = d - 1.08253 P
rounded to um; `head_height` and `Iso4032.height` are the published
maximum with the published minimum as `[+0 -x]`; `head_dia`, `socket`
and `width_across_flats` are the nominal (maximum) value; `TapDrill`
sizes are the nearest ISO 2306 preferred drills. As with retaining
rings, figures were entered from recall (least certain: counterbores for
M1.6 to M2.5, `cut_65` and `form` drills, nut height minima) and the
std citation audit must verify them.

Values are stated as the standard publishes them. A dimension published
as a limit is stored with that tolerance (`head_height` is
`[+0 -x]` from its maximum; `Din472.groove_dia` and `groove_width`
carry their tolerance classes), so checks against them keep their real
margins. Sizes we make (`Iso273` holes, `counterbore_dia`, tap drills)
are nominal and untoleranced: the hole's tolerance is the user's (a
class, or the CAD default), and M003 applies when it is a fit side.

Part families read these: `std::fasteners::Iso4762(thread, length, unit_cost?)` has
members `thread major pitch length head_dia head_height mass` and
implements `std::fasteners::Screw`. It accepts any length; preferred
lengths only constrain what `ScrewJoint` picks.

`Screw` (`std/src/fasteners/screw.cdt`) has members `thread major pitch
length head_dia head_height mass`. `Iso4762.mass` is a design-stage
estimate: a shank cylinder of the thread major diameter and the length
plus a head cylinder of `head_dia` by `head_height`, in steel at 7.85
g/cm^3 (no socket or thread relief, so it slightly overstates), with pi
written `180deg / 1rad`. The density was entered from recall and is
flagged for the citation audit.

## 3. Screw joints: "screw here"

```
use std::fasteners::ScrewJoint

assembly Box {
  part lid: Lid
  part base: Base
  part lid_screws: ScrewJoint(thread = M4, clamp = lid.wall, into = base.material,
                              at = lid.holes, onto = base.tapped)
}
```

One line per hole pattern, and every derived dimension is a member of the joint with a
canonical path (`acme.box::Box/lid_screws.clearance_hole`), bindable
straight to the hole features in each part's CAD file:

| Member | Computed from |
|---|---|
| `clearance_hole` | `Iso273[thread]` at the joint's `clearance` class (default `medium`) |
| `counterbore_dia`, `counterbore_depth` | the head's table (`Iso4762Head`, `Iso7380Head` or `Iso10642Head` by `head`), when `counterbore = true` |
| `min_engagement` | `Materials[into].engagement_factor * thread.major` (steel 1.0, brass and cast iron 1.5, aluminium 2.0, ...) |
| `grip` | `clamp` minus `counterbore_depth` when counterbored: the material actually under the head |
| `screw_length` | `Iso4762Len.at_least(grip + min_engagement).key` |
| `part screw[count(at)]` | `Iso4762(thread, screw_length)`, one per hole, so the BOM and mass roll-ups count one screw per hole (cost: section 4); `count` instead of `count(at)` when there is no pattern |
| `float` | `Length`: the allowance the joint's `aligned` check uses (geometry.md 5.2); a non-locating joint carries it, with a rotation bound, into the placement edge between its two parts |
| `tap_drill` | `TapDrill[thread]` column chosen by `Materials[into].tap_class` and `tap` (`Cut` or `Form`): hard materials get the larger 65% drill |
| `thread_depth` | tip penetration `screw_length - grip` plus the policy's bottom clearance |
| `drill_depth` | `thread_depth` plus `tap_lead_in * thread.pitch` and `drill_point * tap_drill` |
| `callout` | Text: `fmt("{} - 6H, {} DP; drill {} DP {}", thread, thread_depth, tap_drill, drill_depth)`, for drawings and emitted comments |

Checks inside the joint, with the names their verdicts carry
(`Mount/screws.engaged`): `engaged` (engagement is at least
`min_engagement`), `not_bottomed` (`thread_depth > screw_length -
grip`), `grip_positive`, `depth_ok` (only with `depth_available`) and
`aligned` (only with `at` and `onto`, geometry.md 5). Optional arguments:

- `nut` (`Bool`, default `false`): `onto` is a second clearance pattern, so `float` sums both halves.
- `head` (`SocketHead` default, `ButtonHead`, `FlatHead`), `clearance`
  (`fine medium coarse`), `counterbore`, `policy`.
- `tap` (`Cut` default, `Form`, `Vendor`). `Vendor` means the threaded
  hole already exists (a bought part, a threaded insert): `tap_drill`,
  `drill_depth` and the callout's drill clause are absent, and
  `depth_available` is required.
- `depth_available`: adds a check that `drill_depth` (or, for `Vendor`,
  `thread_depth`) fits.
- `host`: the part that carries the threaded hole (`host = base`), so
  `tap_drill`, `thread_depth`, `drill_depth` and `callout` bind into
  that part's CAD file; `into` defaults to `host.material` when it has
  one. Without `host`, the joint's hole members bind like any assembly
  value.
- `screw_cost` (Money, cited where written): becomes the screw part's
  `unit_cost`, so cost roll-ups include screws (section 4).
- `at` and `onto`: the clearance-hole positions in the clamped part and
  the threaded-hole positions in the host (geometry.md 5). With both,
  the joint also checks that the holes line up, using the fixed-fastener
  allowance, or the floating one with `nut = true` (then `onto` is the
  second part's clearance holes).
- `count: Count` (default 1): the number of screws when there is no
  pattern. A joint with `at` and `onto` takes its count from `count(at)`
  (G004 when `count(onto)` differs); `count` is then not allowed.
- `locating: Bool` (default false): true when a pilot, dowel or fit
  locates the clamped part. Otherwise, when the clamped part and the
  host are placement parent and child, that edge gets an implicit
  `[+-float]` on x and y and a rotation bound about the pattern's
  centroid (geometry.md 5.2); G005 warns when a check tighter than the
  float reads through it, and G008 when a one-point pattern leaves the
  rotation free.

A joint is one part per hole pattern, never one per hole: `part
screws[4]: ScrewJoint(..., at = holes)` is E006 (argument mismatch),
because each instance would check the whole pattern. The joint owns its screws
internally.
The rule is checked on the part site (any array slot whose callee takes a
`Pattern` argument that the slot supplies) and as `#![forbids(count, when =
given(at))]` in the joint, so `count` with `at` is E006 too.

A heat-set insert in 0.1.0 is `tap = Vendor` with `into = Brass360` and
`depth_available` the insert's length; the insert's own hole is a `fit`
(language.md 3.7). Models read joint outputs freely as long as no
member reads itself (D003 is per member, language.md 4).

`ScrewJoint` lives in `std/src/fasteners/screwjoint.cdt` with the enums
`ClearanceClass` (`Fine Medium Coarse`), `TapKind` (`Cut Form Vendor`) and
`HeadKind` (`SocketHead ButtonHead FlatHead`). Decisions:
members are public unless written `priv`, so the public members are exactly
the table above plus the arguments and the intermediates (`major`, `pitch`,
`pocket_dia`, `pocket_depth`, `column`, `drill_nominal`, `material`, `n`, `depth_needed`)
are `priv`; the shop numbers (engagement factor, tap
column, preferred lengths, drills, depth allowances) are read from the
`policy` argument; without `counterbore` the pocket members are `0mm`, so
`grip` is `clamp`; `float` is the fixed-fastener allowance `(clearance_hole
- major) / 2`; the tap drill is the policy's column rounded `at_least` into
`policy.drills`.

Arguments without a neutral value are optional parameters, written
`name?: Type` (language.md 3.3): `into`, `host`, `count`, `depth_available`,
`screw_cost`, `at` and `onto`. An omitted optional argument is absent, and
`given(name)` tests for it. The joint's `material` is `into` when given,
else `host.material`; the screw count `n` is `count(at)`, else `count`,
else 1. Presence rules are attributes (language.md 3.11): `#[when(cond)]`
on a member or check makes it exist only while `cond` holds (reading an
absent member is E002), and `#![requires(arg, when = cond)]` makes an
argument mandatory under a condition (E006 when missing). So `tap_drill`,
`drill_depth`, `column` and `drill_nominal` carry `#[when(tap !=
TapKind::Vendor)]`, `depth_ok` carries `#[when(given(depth_available))]`,
`aligned` carries `#[when(given(at) and given(onto))]`, and `tap = Vendor`
requires `depth_available`. `head` is an argument, so the internal head
pocket members are `pocket_dia` and `pocket_depth`, chosen by `head`. `Iso4762` takes an optional `unit_cost`, which
the joint fills from `screw_cost`. The `engaged` check is
`screw_length - grip >= min_engagement`; the other checks are as listed
above. `float` is one clearance hole's half, `(clearance_hole - major) / 2`,
taken from the ISO 273 class nominal and independent of any tolerance on
the user's hole; with `nut = true` (`onto` is a second clearance pattern)
both halves count, so `aligned` gets one half fixed and both halves with
a nut. `locating` is accepted, but the placement float it
removes is not yet applied to a joint's edge: `cadetta-geometry`
can evaluate it (geometry.md 5.2) and the lowering from a joint is
still open. `ButtonHead` (ISO 7380) and `FlatHead` (ISO 10642) have counterbore
data for the sizes their standards cover (M3 to M16 and M3 to M20);
`counterbore = true` on another size is E007 (no row). The ISO 7380
pocket is derived from the head, and the ISO 10642 pocket is the
countersink, flush with the head.

Change `into = base.material` from aluminium to stainless and the tap
drill, engagement, screw length, thread and drill depths and callout all
follow; `cadetta impact` shows which CAD files need a re-sync.

## 4. Shop policy

The numbers a shop disagrees about live in a `FastenerPolicy`:

```
interface FastenerPolicy {
  engagement: Table<Material>        // column `factor: Ratio`; read as policy.engagement[into].factor, replaces Materials.engagement_factor
  tap_columns: Table<TapClass>       // columns `cut, form: TapColumn`: which TapDrill column
  lengths: Table<Length>             // preferred screw lengths (key-only table)
  drills: Table<Length>              // drills on the shelf; tap drills round at_least into it
  bottom_clearance: Length
  tap_lead_in: Count                 // in pitches: drill depth adds tap_lead_in * thread.pitch
  drill_point: Ratio                 // point length per drill diameter (0.3 for 118deg)
}
```

Table-typed members (language.md 3.3) are how `std` passes rules as
data; user interfaces may use them too. Screw prices are not policy:
`Iso4762` takes an optional `unit_cost`, so screws are listed as missing by
`sum?(self, unit_cost)` until the joint is given `screw_cost`.
`std::fasteners::IsoDefault` is the cited default and the template to
copy: engagement factors and tap columns from `Materials`, lengths from
`Iso4762Len`, drills from the ISO 2306 preferred series,
`bottom_clearance = 1mm`, `tap_lead_in = 3` (pitches, a plug tap) and
`drill_point = 0.3` (118deg), each cited to Machinery's Handbook. `FastenerPolicy` and `IsoDefault` live in `std/src/fasteners/policy.cdt`,
with `TapColumn` (`Cut75 Cut65 Form`). Decisions: a table-typed member
takes a table by name, so `IsoDefault` points `lengths` at `Iso4762Len`
and carries `IsoEngagement` (column `factor`, repeating
`Materials.engagement_factor`) and `IsoTapColumns` (`Soft` takes
`Cut75`, `Hard` takes `Cut65`, both `Form` for forming) because
`FastenerPolicy` fixes the column names; `IsoDrills` is the union of the
`TapDrill` sizes (derived, not transcribed from ISO 2306, and flagged
for the citation audit). The policy figures were entered from recall.
A project overrides it
for a whole scope without touching call sites:

A shop keeps its policy in a library package, `acme.shop`, so every
product shares one definition by version (packages.md). Items other
packages use are `pub`:

```
// acme.shop, src/policy.cdt
pub ots AcmeShop: FastenerPolicy @"shop standard SS-104 rev C" { ... }

// acme.shop, src/lib.cdt: present a curated surface
pub use self::policy::AcmeShop
```

A product that depends on `acme.shop` then says:

```
use acme.shop::AcmeShop

#![default(FastenerPolicy = AcmeShop)]      // every ScrewJoint in this file
```

`#![default(Iface = Impl)]` sets the default for every component
parameter of that interface type in its scope (language.md 3.11). A
package sets it once for all its modules in `cadetta.toml`:

```
[defaults]
FastenerPolicy = "acme.shop::AcmeShop"       # dependency path
```

Precedence, highest first: the component's `#![default]`, the module's
`#![default]`, the package `[defaults]`, then the `std` default
(`IsoDefault`). No lint fires on a fallback to `IsoDefault`.

## 5. CAD hole features

In 0.1.0 a joint's members are plain parameters: adapters emit and bind
them like any other value, and the callout travels as Text and as a
trailing emitted comment. Driving native hole features (SOLIDWORKS Hole
Wizard, FreeCAD PartDesign Hole, Onshape hole) from a joint is OQ-14.

## 6. Commercial data: `std::commercial::Sourced`

```
interface Sourced {
  unit_cost?: Money
  lead_time?: Time
  moq?: Count
  supplier?: Text
  mass?: Mass
}
```

The commercial names are a declared contract, not magic spellings. The
roll-ups (`sum?(self, unit_cost)`, `mass`; language.md 3.5), the
commercial deltas of `impact --swap` and the `compare` columns are
exactly these members. A part family implements `Sourced` (or a
component declares the members); a misspelling such as `leadtime` is
simply not a commercial member. A wrong kind on one of these names in a
component that implements `Sourced` is I001 when the dimension is wrong
(`unit_cost = 3 mm`) and Q003 when a named kind is wrong (`unit_cost`
where `Money` is required and the value is a plain length); a component
that does not implement the interface is not checked. Every member is declared optional
(`unit_cost?: Money`, language.md 3.2), so a component lacking one is
not I001; the `?` roll-up forms list it as missing, never defaulted.

## Conformance decisions

Fixed while writing `tests/conformance/stdlib/` (ticket ~GDYVJ8R):

- `count` together with `at`/`onto` is E006: the pattern fixes the count.
- `tap = Vendor` without `depth_available` is E006 (missing argument).
- Under `tap = Vendor`, reading `tap_drill` or `drill_depth` is E002 (the
  members are absent). The Vendor `callout` is
  `fmt("{} - 6H, {} DP", thread, thread_depth)`.
- The `engaged` check is a defense-in-depth guard, kept on purpose.
  `screw_length` is derived with `at_least`, so for every joint the
  stdlib can build the check holds, and a table with no long-enough
  length fails the lookup itself (a table error), not `engaged`. The
  check therefore cannot fire from any source today and has a
  must-not-fire fixture only; a must-fire fixture is not required. It
  exists so a future `length` override argument, or a hand-edited length
  table policy, is caught without a new check. Adding such an override
  MUST add a must-fire fixture in the same change.
- Owner review (2026-10-16, ~MEM04J7): the four rules above (count with
  pattern E006, Vendor without `depth_available` E006, absent Vendor
  members E002, the Vendor `callout` text) are confirmed as the spec.
- A roll-up (`max`, `sum`) over a misspelled member is E002 (unknown
  member), reported at the roll-up expression, like any other member
  read. It must never contribute nothing silently, and the internal code
  EVAL must not reach users. Implementation conforms in a follow-up.

## Open questions

- OQ-14: Should adapters map a whole joint onto a native hole feature
  (size, standard, depth, thread) rather than numeric parameters?
  Recommendation: yes, after 0.1.0, as an optional adapter capability.
- OQ-15 (closed 2026-10-13, draft 1.3 decisions): inch fasteners and inserts ship in
  `std` in 0.2.0, not as packages; until then an inch joint is
  hand-modelled (pilot r3-06).
