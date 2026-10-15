+++
id = "01M4Z8V9VNHK5QN6G0B0MC1ZA9"
title = "cadetta conformance: library behaviour for the remaining binding read fixtures (R002 unbound, R003 read-bound, R007, R011, Q001 kg-vs-mm, driven dimensions, lock digest, slots)"
type = "task"
category = "todo"
priority = "medium"
points = 8
reporter = "lognd"
created = "2026-10-15T07:53:53Z"
updated = "2026-10-15T07:53:53Z"
scope = ["crates/cadetta-adapter/**", "crates/cadetta-cli/tests/**"]

[[acceptance]]
text = "Each listed fixture executes against the adapter host and its known-gaps.toml entry is removed."
bound = false
+++

found while working W796HD7: ten binding read fixtures need adapter-host behaviour that does not exist: adapters/read-driven-dimension-inexact, read-never-raises-q001, read-unparsable-expression-inexact (SWFN(1) parses as Call), r002-unbound-native-parameter, r003-read-bound-path-missing, read-maps-names-via-bindings-only, lock-digest-skips-open, slots-read-records-instance-file, r011-wrong-component-file, geom-r007-not-from-origin and geometry/r007-not-from-origin.
