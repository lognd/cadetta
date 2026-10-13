# 01-bracket

A machined L-bracket screwed to a wall and clamping a 20mm tube, written to spec draft 1.4. Port of pilots 01 and r3-01.

Exercises: H11 fit against a vendor shaft tolerance (language.md 3.7); design
vs analysis tolerance, parameters vs `let` (quantities.md 5.3); `ScrewJoint`
with `host`, `at`, `onto`, one joint per hole pattern (stdlib.md 3); datums,
`flip` and placement on the wall's pattern datum (geometry.md 2, 4); the
`aligned` check inside each joint (geometry.md 5). The wall is an `ots`
carrying the tapped pattern.

Layout: a `[package]` `acme.bracket`; `src/lib.cdt` holds the bracket and
assembly, `src/parts.cdt` is a small library module with `pub` `Tube` and
`Wall` (lib.cdt re-exports `Tube` with `pub use`), so the wall and tube
paths are `acme.bracket::parts::Tube` and `...::parts::Wall`.

Notable: `Bracket(clamp_hole = clamp_screws.clearance_hole)` feeds a joint
output back into a model, so members reading it have instance paths. Both
`aligned` verdicts hold (0.1mm and 0.1414mm worst case vs 0.25mm allowance).
Std table figures and joint check names are assumptions (see expected.toml).
