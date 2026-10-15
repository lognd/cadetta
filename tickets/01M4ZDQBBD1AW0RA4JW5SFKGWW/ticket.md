+++
id = "01M4ZDQBBD1AW0RA4JW5SFKGWW"
title = "cadetta sync: SessionBackend exposes a Document so live pushes restore on R004"
type = "task"
category = "in-progress"
priority = "medium"
points = 3
reporter = "lognd"
created = "2026-10-15T09:19:06Z"
updated = "2026-10-15T09:22:07Z"
scope = ["crates/cadetta-cli/src/cmd/sync.rs", "crates/cadetta-cli/tests/conformance/sync.rs"]

[[acceptance]]
text = "SessionBackend implements Document over DriveSession so sync push goes through push_atomic and restores on R004"
bound = false

[[acceptance]]
text = "Backend::document is required and Backend::push is removed"
bound = false
+++

Follow-up to ~HPED6RY. sync::execute now routes through push_atomic when Backend::document returns Some, but SessionBackend (cli cmd/sync.rs) and the conformance Sim still implement only Backend::push. Implement Document over the DriveSession (read, push, regen), record R004 and the regen record in Backend::pushed, then make Backend::document required and drop Backend::push. Blocked on lease of ~R77QY6K over cmd/sync.rs.
