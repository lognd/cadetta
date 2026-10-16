+++
id = "01M51KWZMAAKYV02YTE4JZ8B7P"
title = "cmd::check ots_datum_offset_without_citation_is_s001 stale after ~9MJNAM5 component-level S001"
type = "bug"
category = "todo"
priority = "high"
reporter = "lognd"
created = "2026-10-16T05:45:31Z"
updated = "2026-10-16T05:45:31Z"
scope = ["crates/cadetta-cli/src/cmd/check.rs"]
+++

Not a [position d] regression: 3ecdc45a (~9MJNAM5) reports a wholly uncited ots once at the component path, so the test expecting acme.axle::Plate.top.offset.z fails. Fix: cite another member so the datum offset is reported per value.
