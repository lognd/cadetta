# The standard library

`std` ships with the compiler, versioned with it, and is written in
ordinary `.cdt`: every table is a `table`, every part family a
parameterized `ots`, every helper a `fn` or a parameterized `assembly`.
Nothing in `std` needs a language feature user code cannot use; if it
did, that feature would be missing from the language. Every value in
`std` is cited to its standard or handbook (S001 applies to `std` too).

Why a library and not syntax: a screw is shop knowledge (clearance
classes, tap drills, engagement rules, preferred lengths), and shops
disagree. Data can be cited, overridden and diffed; syntax cannot.

## 1. Modules

| Module | Contents | 0.1.0 |
|---|---|---|
| `std.fits` | ISO 286 classes (`iso286`) and the fit machinery behind `fit` | yes |
| `std.materials` | `Material` enum and the `Materials` table: density, yield, tap class, engagement factor | yes, a short list |
| `std.fasteners` | Metric threads, ISO 4762 socket head cap screws, ISO 4032 nuts, ISO 273 clearance holes, tap drills, preferred lengths, `ScrewJoint` | yes, metric coarse |
| `std.retaining` | DIN 471/472 retaining rings and grooves | yes |
| `std.bearings` | Deep-groove ball bearing envelopes (ISO 15) | later |
| `std.fasteners.inch` | UNC/UNF, ASME B18.3, inch clearance and tap drills | later |
| `std.inserts` | Heat-set and wire-thread inserts | later |

Paths are `std.module::Item` like any namespace (language.md 3).

## 2. Tables and their columns

Column names are part of `std`'s interface: renaming one is a breaking
change and is recorded with `was`.

| Table | Key | Columns |
|---|---|---|
| `std.fasteners::Iso273` | `MetricThread` | `fine medium coarse` (clearance hole diameters) |
| `std.fasteners::Thread` | `MetricThread` | `major pitch minor_ext minor_int` |
| `std.fasteners::TapDrill` | `MetricThread` | `cut_75 cut_65 form` (cutting tap at about 75% and 65% thread, forming tap) |
| `std.fasteners::Iso4762Head` | `MetricThread` | `head_dia head_height socket counterbore_dia counterbore_depth` |
| `std.fasteners::Iso4762Len` | `Length` | (preferred lengths; key only) |
| `std.fasteners::Iso4032` | `MetricThread` | `width_across_flats height` |
| `std.materials::Materials` | `Material` | `density yield_strength tap_class engagement_factor` |
| `std.retaining::Din471` / `Din472` | `Length` (shaft or bore) | `groove_dia groove_width ring_thickness` |

Part families read these: `std.fasteners::Iso4762(thread, length)` has
members `thread major pitch length head_dia head_height mass unit_cost?`
and implements `std.fasteners::Screw`.

## 3. Screw joints: "screw here"

```
use std.fasteners::ScrewJoint

assembly Box {
  part lid: Lid
  part base: Base
  part lid_screws[4]: ScrewJoint(thread = M4, clamp = lid.wall, into = base.material)
}
```

One line, and every derived dimension is a member of the joint with a
canonical path (`acme.box::Box/lid_screws.clearance_hole`), bindable
straight to the hole features in each part's CAD file:

| Member | Computed from |
|---|---|
| `clearance_hole` | `Iso273[thread]` at the joint's `clearance` class (default `medium`) |
| `counterbore_dia`, `counterbore_depth` | `Iso4762Head[thread]`, when `counterbore = true` |
| `min_engagement` | `Materials[into].engagement_factor * thread.major` (steel 1.0, brass and cast iron 1.5, aluminium 2.0, ...) |
| `screw_length` | `Iso4762Len.at_least(clamp + min_engagement).key` |
| `part screw` | `Iso4762(thread, screw_length)`, so it is in the BOM, mass and cost roll-ups |
| `tap_drill` | `TapDrill[thread]` column chosen by `Materials[into].tap_class` and `tap` (cut or form): hard materials get the larger 65% drill |
| `thread_depth` | tip penetration `screw_length - clamp` plus the policy's bottom clearance |
| `drill_depth` | `thread_depth` plus the policy's tap lead-in and the drill point |
| `callout` | Text: `fmt("{} - 6H, {} DP; drill {} DP {}", thread, thread_depth, tap_drill, drill_depth)`, for drawings and emitted comments |

Checks inside the joint: engagement is at least `min_engagement`; the
screw does not bottom out (`thread_depth > screw_length - clamp`);
`clamp` is positive. Optional arguments: `head` (`SocketHead` default,
`ButtonHead`, `FlatHead`), `clearance` (`fine medium coarse`), `tap`
(`cut` default, `form`), `counterbore`, `depth_available` (adds a check
that `drill_depth` fits), `policy`.

Change `into = base.material` from aluminium to stainless and the tap
drill, engagement, screw length, thread and drill depths and callout all
follow; `cadetta impact` shows which CAD files need a re-sync.

## 4. Shop policy

The numbers a shop disagrees about live in a `FastenerPolicy`: thread
percentage per tap class, engagement factors, preferred lengths, bottom
clearance, tap lead-in, and the drill set available (tap drills round
`at_least` into it). `std.fasteners::IsoDefault` is the cited default. A
project overrides it for a whole scope without touching call sites:

```
ots AcmeShop: FastenerPolicy @"shop standard SS-104 rev C" { ... }

#![default(FastenerPolicy = AcmeShop)]      // every ScrewJoint in this file
```

`#![default(Iface = Impl)]` sets the default for every component
parameter of that interface type in its scope (language.md 3.11).

## 5. CAD hole features

In 0.1.0 a joint's members are plain parameters: adapters emit and bind
them like any other value, and the callout travels as Text and as a
trailing emitted comment. Driving native hole features (SOLIDWORKS Hole
Wizard, FreeCAD PartDesign Hole, Onshape hole) from a joint is OQ-14.

## Open questions

- OQ-14: Should adapters map a whole joint onto a native hole feature
  (size, standard, depth, thread) rather than numeric parameters?
  Recommendation: yes, after 0.1.0, as an optional adapter capability.
- OQ-15: Inch fasteners and inserts in `std`, or as first-party packages
  once packages exist (OQ-11)?
