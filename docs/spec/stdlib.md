# The standard library

`std` ships with the compiler, versioned with it, and is written in
ordinary `.cdt`: every table is a `table`, every part family a
parameterized `ots`, every helper a `fn` or a parameterized `assembly`.
Nothing in `std` needs a language feature user code cannot use; if it
did, that feature would be missing from the language. Every value in
`std` is cited to its standard or handbook (S001 applies to `std` too).
`std` exports one `cite` per standard at its root (`std::iso273`,
`std::iso286`, `std::din472`, ...), so user code cites a standard with
`cite din472 = std::din472`. Library intermediates are `priv` members
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
| `std::inserts` | Heat-set and wire-thread inserts | 0.2.0 |
| `std::fasteners` thread-forming | ISO 7049/7085 thread-forming screws and pilot holes | later |

Paths are `std::module::Item`, and `std` is an implicit dependency
(packages.md 1). Everything it exports is `pub` (packages.md 3).

## 2. Tables and their columns

Column names are part of `std`'s interface: renaming one is a breaking
change and is recorded with `was`.

| Table | Key | Columns |
|---|---|---|
| `std::fasteners::Iso273` | `MetricThread` | `fine medium coarse` (clearance hole diameters) |
| `std::fasteners::Thread` | `MetricThread` | `major pitch minor_ext minor_int` |
| `std::fasteners::TapDrill` | `MetricThread` | `cut_75 cut_65 form` (cutting tap at about 75% and 65% thread, forming tap) |
| `std::fasteners::Iso4762Head` | `MetricThread` | `head_dia head_height socket counterbore_dia counterbore_depth` |
| `std::fasteners::Iso4762Len` | `Length` | (preferred lengths; key only) |
| `std::fasteners::Iso4032` | `MetricThread` | `width_across_flats height` |
| `std::materials::Materials` | `Material` | `density yield_strength tap_class engagement_factor` |
| `std::retaining::Din471` / `Din472` | `Length` (shaft or bore) | `groove_dia groove_width ring_thickness edge_margin` (each with its published tolerance) |

`Material` variants in 0.1.0: `Al6061T6 Al7075T6 Al5052H32 Ss304 Ss316
Steel1018 Steel4140 CastIron Brass360 Petg Pla Abs Nylon6 Pom`. Each row
is cited to a materials handbook. `MetricThread` variants: `M1_6 M2
M2_5 M3 M4 M5 M6 M8 M10 M12 M16 M20` (an underscore between digits is
the decimal mark; `M2_5` displays as `M2.5`). `TapClass`: `Soft Hard`.
`TapColumn`: `Cut75 Cut65 Form`, naming the `TapDrill` columns
`cut_75 cut_65 form`.

Values are stated as the standard publishes them. A dimension published
as a limit is stored with that tolerance (`head_height` is
`[+0 -x]` from its maximum; `Din472.groove_dia` and `groove_width`
carry their tolerance classes), so checks against them keep their real
margins. Sizes we make (`Iso273` holes, `counterbore_dia`, tap drills)
are nominal and untoleranced: the hole's tolerance is the user's (a
class, or the CAD default), and M003 applies when it is a fit side.

Part families read these: `std::fasteners::Iso4762(thread, length)` has
members `thread major pitch length head_dia head_height mass` and
implements `std::fasteners::Screw`. It accepts any length; preferred
lengths only constrain what `ScrewJoint` picks.

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
| `counterbore_dia`, `counterbore_depth` | `Iso4762Head[thread]`, when `counterbore = true` |
| `min_engagement` | `Materials[into].engagement_factor * thread.major` (steel 1.0, brass and cast iron 1.5, aluminium 2.0, ...) |
| `grip` | `clamp` minus `counterbore_depth` when counterbored: the material actually under the head |
| `screw_length` | `Iso4762Len.at_least(grip + min_engagement).key` |
| `part screw[count(at)]` | `Iso4762(thread, screw_length)`, one per hole, so the BOM and mass roll-ups count one screw per hole (cost: section 4); `count` instead of `count(at)` when there is no pattern |
| `float` | `Length`: the allowance the joint's `aligned` check uses (geometry.md 5.2); a non-locating joint carries it into the placement it implies |
| `tap_drill` | `TapDrill[thread]` column chosen by `Materials[into].tap_class` and `tap` (`Cut` or `Form`): hard materials get the larger 65% drill |
| `thread_depth` | tip penetration `screw_length - grip` plus the policy's bottom clearance |
| `drill_depth` | `thread_depth` plus `tap_lead_in * thread.pitch` and `drill_point * tap_drill` |
| `callout` | Text: `fmt("{} - 6H, {} DP; drill {} DP {}", thread, thread_depth, tap_drill, drill_depth)`, for drawings and emitted comments |

Checks inside the joint, with the names their verdicts carry
(`Mount/screws.engaged`): `engaged` (engagement is at least
`min_engagement`), `not_bottomed` (`thread_depth > screw_length -
grip`), `grip_positive`, `depth_ok` (only with `depth_available`) and
`aligned` (only with `at` and `onto`, geometry.md 5). Optional arguments:

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
  locates the clamped part. Otherwise a part placed on its host's
  pattern datum gets an implicit `[+-float]` on x and y (one source
  variable pair per joint), and G005 warns when a check tighter than
  the float reads through it (geometry.md 5.2).

A joint is one part per hole pattern, never one per hole: `part
screws[4]: ScrewJoint(..., at = holes)` is E006 (argument mismatch),
because each instance would check the whole pattern. The joint owns its screws
internally.

A heat-set insert in 0.1.0 is `tap = Vendor` with `into = Brass360` and
`depth_available` the insert's length; the insert's own hole is a `fit`
(language.md 3.7). Models read joint outputs freely as long as no
member reads itself (D003 is per member, language.md 4).

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
`Iso4762` has no `unit_cost`, so screws are listed as missing by
`sum?(self, unit_cost)` until the joint is given `screw_cost`.
`std::fasteners::IsoDefault` is the cited default and the template to
copy: engagement factors and tap columns from `Materials`, lengths from
`Iso4762Len`, drills from the ISO 2306 preferred series,
`bottom_clearance = 1mm`, `tap_lead_in = 3` (pitches, a plug tap) and
`drill_point = 0.3` (118deg), each cited to Machinery's Handbook. A project overrides it
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
  unit_cost: Money
  lead_time: Time
  moq: Count
  supplier: Text
  mass: Mass
}
```

The commercial names are a declared contract, not magic spellings. The
roll-ups (`sum?(self, unit_cost)`, `mass`; language.md 3.5), the
commercial deltas of `impact --swap` and the `compare` columns are
exactly these members. A part family implements `Sourced` (or a
component declares the members); a misspelling such as `leadtime` is
simply not a commercial member, and a wrong kind on one of these names
is an ordinary Q003. Every member is declared optional
(`unit_cost?: Money`, language.md 3.2), so a component lacking one is
not I001; the `?` roll-up forms list it as missing, never defaulted.

## Open questions

- OQ-14: Should adapters map a whole joint onto a native hole feature
  (size, standard, depth, thread) rather than numeric parameters?
  Recommendation: yes, after 0.1.0, as an optional adapter capability.
- OQ-15 (closed 2026-10-13, draft 1.3 decisions): inch fasteners and inserts ship in
  `std` in 0.2.0, not as packages; until then an inch joint is
  hand-modelled (pilot r3-06).
