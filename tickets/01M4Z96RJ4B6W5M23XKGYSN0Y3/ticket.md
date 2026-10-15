+++
id = "01M4Z96RJ4B6W5M23XKGYSN0Y3"
title = "cadetta adapter: discover() warns R002 for a dimension whose bare reference names a global that is not a bound native global"
type = "bug"
category = "todo"
priority = "medium"
points = 2
reporter = "lognd"
created = "2026-10-15T08:00:08Z"
updated = "2026-10-15T08:00:08Z"
scope = ["crates/cadetta-adapter/src/bindings.rs"]

[[acceptance]]
text = "native-link-dimension-link passes with no known-gaps entry."
bound = false
+++

found while working W796HD7: fixture adapters/native-link-dimension-link must-not-fire (equation "Other", unrelated global) expects no diagnostics, but bindings::discover emits R002 whenever the equation is not a link. Decide whether a bare reference to a non-bound name is silent per adapters.md 3.5, fix discover, then drop the known-gaps entry.
