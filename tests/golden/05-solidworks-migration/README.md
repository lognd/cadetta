# 05 SOLIDWORKS migration

A gearbox cover (plate, gasket, four M6 cap screws) migrated from a legacy
SOLIDWORKS global-equations file, `input/legacy-equations.txt`, into a cleaned
Cadetta design (`src/lib.cdt`) with accepted bindings in
`bindings/sw-equations.toml`.

Exercises: unit-bearing literals and assumed bare numbers (quantities.md 5.3,
5.6), H7 fit class (language.md 3.7), std lookups (stdlib.md 2), locator
bindings with `ref_to`, alias paths and mangled globals (adapters.md 3, 3.1),
and the byte-exact equations emitter (emitters/sw-equations.md).

Notable: the emitted file's header digests are placeholders (`<hex>`), the
inch-native thickness and length are written in inches, the H7 bore is emitted
at its midpoint (12.009mm), and no `proposal.toml` is pinned because
adapters.md 7 does not define it precisely enough.
