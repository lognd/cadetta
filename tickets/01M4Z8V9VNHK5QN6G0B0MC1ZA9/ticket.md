+++
id = "01M4Z8V9VNHK5QN6G0B0MC1ZA9"
title = "cadetta conformance: library behaviour for binding read fixtures (R002/R003 reconcile, R007, R011, lock digest, slots)"
type = "task"
category = "in-progress"
priority = "medium"
points = 8
reporter = "lognd"
created = "2026-10-15T07:53:53Z"
updated = "2026-10-15T19:57:42Z"
scope = ["crates/cadetta-adapter/**", "crates/cadetta-cli/tests/**", "tests/conformance/known-gaps.toml", "docs/**"]

[[acceptance]]
text = "Reconciling native names against bindings reports R002 for an unbound name, R003 for a bound but absent one, and maps names via bindings only."
bound = true

[[acceptance]]
text = "R007 fires for a dimension not measured from the sketch origin and R011 for an instance file that is not the slot file; a slots read records instance files."
bound = true

[[acceptance]]
text = "A read whose on-disk digest equals the lock digest reuses the lock values without opening the tool."
bound = true
+++

found while working W796HD7: ten binding read fixtures need adapter-host behaviour that does not exist: adapters/read-driven-dimension-inexact, read-never-raises-q001, read-unparsable-expression-inexact (SWFN(1) parses as Call), r002-unbound-native-parameter, r003-read-bound-path-missing, read-maps-names-via-bindings-only, lock-digest-skips-open, slots-read-records-instance-file, r011-wrong-component-file, geom-r007-not-from-origin and geometry/r007-not-from-origin.
