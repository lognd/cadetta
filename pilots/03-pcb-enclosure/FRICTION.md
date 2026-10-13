# Friction report: Raspberry Pi 4B enclosure (pilot 03)

## 1. Summary

Scalars, units, exact arithmetic and `round_to` worked well; the derivation
chain datum -> standoff/cutout centres is legible and checkable. The design
is spec-legal but verbose: 2D positions, repeated patterns and process
compensation have no representation, so ~60 parameters do the work of ~15
declarations and the key manufacturing rule (add compensation to every hole)
is enforced only by author discipline. Only 3 lines needed `PROPOSED`-style
workarounds; most pain is structural, not syntactic.

## 2. Findings

F1 (awkward-to-blocker, language.md 3, quantities.md 1). No point/vector/array.
Wanted: `hole[4]: Point2 = [(3.5mm,3.5mm),...]`. Forced: `hole_a_x`..`hole_d_y`,
and every cutout is `*_cut_x/_z/_width/_height`. Suggest
`type Point2 { x: Length, y: Length }` and `pattern holes = rect(58mm, 49mm) at (3.5mm, 3.5mm)`
yielding `holes[0..3].x`, with `count(holes)` as a Count.

F2 (awkward, language.md 4). No coordinate frames. `Case.datum_x + Pi.foo_x`
is typed 16 times; forgetting it on one is silent. Suggest
`frame pi at (Case.datum_x, Case.datum_y)` and `Pi.usbc_x in frame base`.

F3 (blocker for process work, quantities.md 5, language.md 3.3). FDM
compensation has no home. It is not a tolerance (it shifts the drawn value,
it is not a band), not a derivation of a single parameter (it applies to a
class of features), not config (it belongs to the design). I modelled it as
`ots Fdm` and added `Fdm.hole_undersize` by hand at each hole; sign differs
for holes (+), bosses (-), slots, and mating lip faces. Nothing catches a
missed one. Suggest `process Fdm { hole += 0.2mm; boss -= 0.1mm }` and
`diameter: Length = 3.2mm @process(hole)`; emitter then writes drawn value
and keeps the as-printed intent in the IR.

F4 (awkward, language.md 3.1, lints D002). `ots` forbids expressions, so
`hole_b_x = 61.5mm` restates `hole_a_x + hole_pitch_x`; D002 fires on exactly
what the drawing states. And "Fdm" is a measured process, not an OTS part:
`ots` is the wrong class for facts we measured. Suggest allowing `derive`d
members in `ots`, and a `process` class (see F3).

F5 (awkward, quantities.md 5.1-5.2). `hole_undersize = 0.2mm +- 0.05mm` makes
every dependent a toleranced value; `round_to` on a toleranced quantity is
undefined (I wrote it anyway on cutout sizes). Also no way to say a position
is toleranced (hole 3.5mm from edge) separately from its size.

F6 (nit, quantities.md 2, 3). `%` for shrinkage/infill works as `Ratio`, but
applying shrinkage (`x * (1 + shrinkage)`) mixes anonymous `1` with Ratio;
the spec says anonymous is fine for `+` but is `0.4%` = `4/1000` stated? Unused
parameters (infill, shrinkage, nozzle layer) get no lint, so they rot.

F7 (nit, language.md 3.3). `source` is only a free string; "measured by
caliper" vs "datasheet page 2" vs "assumed" are indistinguishable. PCB
thickness is absent from the official drawing and I had to say so in prose.
Suggest `source` kinds: `datasheet | measured | assumed`, S001 accepts all
but M-lint downgrades `assumed`.

F8 (awkward, language.md 3.2). Cannot express a press/melt-in fit. The
heat-set insert hole (3.4mm drawn for a 3.6mm insert) is an interference, but
`fit` is ISO 286 only, so the interference is an unverified literal pair.
Suggest `fit insert = interference(0.1mm..0.4mm) between Case.insert_hole and Insert.outer_diameter`.

F9 (nit, language.md 3). Counts: `standoff_count: Count = 4` is a fact the
geometry does not use; nothing ties it to the 4 hand-written holes. A
`count(holes)` (F1) would.

F10 (nit, language.md 2). Lexer: `0.4%` after `+` and `3.5mm`; fine. But `3.5mm`
overmold allowances are bare quantities with no `source`/doc, and no lint asks.

## 3. Hypothetical `cadetta check`

- D002 x5 (hole_b..d coordinates, usb/eth offsets): noise (F4); the drawing
  states them.
- D001 on `Case.lid_screw_hole`'s 2.5mm vs `Insert.thread_diameter`: useful
  (should reference it). D001 on 0.4mm vs `Fdm.nozzle`: wrong (coincidence).
- D001 info (1mm, 2mm common values): noise, correct to downgrade.
- M003 x~8 (checks in `PiCase` read untoleranced params of other components):
  noise; every dimension here is a plain number.
- M001: none (all <=2 decimals). M002: none (`2 *`, `4 *`). Silent misses are
  the magic allowances `+ 3.5mm`, `+ 4.5mm`, `+ 3mm` in cutout sizes: no lint.
- S001: none. Q-codes: none after declaring `perimeters: Count`.
- C001/C002: none; all five checks `holds`.

Uncaught engineer mistakes: omitted compensation on one hole; wrong sign;
Y axis flipped (viewing Pi from underside); cutout on the wrong wall; missing
SD-card, GPIO, camera/display slot cutouts (model has no notion that a
connector list is complete); lid/base datum drift; `board_top_z` omitting
PCB thickness.

## 4. Evaluation trace

1. `wall = 6 * 0.45mm`: 0.45mm = 9/20000 m; times 6 = 27/10000 m = 2.7mm,
   exact. `Count * Length` kind is anonymous Length, assigned to
   `wall: Length` (4.3, 4.4).
2. `datum_x = 2.7mm + 1mm = 3.7mm`; `standoff_b_x = 3.7 + 61.5 = 65.2mm`
   (13/200 m). `board_top_z = 2.4 + (4+1) + 1.4 = 8.8mm`; `usb_cut_z = 8.8 + 15.6/2 = 16.6mm`.
3. `usbc_cut_width = round_to(9 + 3.5 + (0.2 +- 0.05), 0.1mm)` =
   `round_to(12.7 +- 0.05mm, 0.1mm)`. Ambiguous: round nominal and keep
   +-0.05? round both limits? Is the result exact? I assumed 12.7 +- 0.05.
4. `mounting_pattern`: 3.5 + 49 = 52.5 and 56 - 3.5 = 52.5; `==` over exact
   rationals holds.

Ambiguities: `round_to` on toleranced input; `%` literal magnitude; precedence
of kind for `Count * Length` in a `check` (`4 * Fdm.nozzle`); whether `round_to`
step must share the unit of the argument.
