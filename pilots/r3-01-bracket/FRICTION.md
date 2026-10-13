# Pilot r3-01 bracket (draft 1.2) friction

## 1. Verdict
The bracket now expresses cleanly: both screw sets, clearance holes,
counterbores, tap drills, lengths and engagement come from `ScrewJoint`;
`let` carries edge distances. 1 PROPOSED line. No spec blocker.

## 2. Prior findings
- F1 ISO 273 lookup: RESOLVED (`Iso273[thread]` via ScrewJoint)
- F2 tolerance leaking into sizes: RESOLVED (quantities 5.3, `.max`)
- F3 max-only dimensions: PARTIAL (std tables hide it; a user `ots` still needs `+0 -x` with nominal=max)
- F4 screw families: RESOLVED (`Iso4762(thread, length)`)
- F5 digits in names: RESOLVED (language 4)
- F6 string metadata: RESOLVED (`Text`)
- F7 bare `== 2`, counts: RESOLVED (array size `[bracket.mount_hole_count]`)
- F8 checks in model: RESOLVED (grammar)
- F9 non-ISO shaft side: RESOLVED (`H11 on Tube.od`; tube may carry vendor tolerance)
- F10 Stress kind: RESOLVED (`kind Stress`, 3.10)

## 3. New findings
1. awkward, language 3.1 vs 3.3/quantities 5.3: the assembly row lists "parameters, parts, fits, checks, requires" but omits `let`, which 5.3 says assembly stack-ups use. Fix: add `let` to the row.
2. awkward, language 3.1/3.5: a model cannot read joint outputs for its own features without a part-level cycle, so `clamp_bolt_offset` had to move to the assembly. Fix: state whether D003 is member-level (so `Bracket(hole = joint.x)` is legal) or allow joints inside models.
3. awkward, stdlib 3: `into` takes a Material only; the tapped part is unnamed, so `tap_drill`/`callout` have no CAD file to bind to (PROPOSED line). Fix: optional `host` argument.
4. awkward, language 3.5: `part x[expr]` with a param-derived size vs the "graph known without evaluating" promise; and no `joints.clearance_hole` over an array (had to write `[0]`). Fix: clarify that array size is a static Count expression; allow uniform-member read on arrays.
5. nit, language 3: no rule for evaluating a parameterized root assembly (`WallTubeMount(wall_material)`) with no argument. Fix: say it is undecided/skipped or require defaults.
6. nit, stdlib 2: unclear whether `Iso4762Head.counterbore_dia` includes slack or is a nominal needing a tolerance (M003/M001 unspecified). Fix: state it.
