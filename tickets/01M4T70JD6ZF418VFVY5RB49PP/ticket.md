+++
id = "01M4T70JD6ZF418VFVY5RB49PP"
title = "Wire the obligation universe (IR closure digests) into cadetta ack"
type = "story"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T08:45:36Z"
updated = "2026-10-13T08:45:36Z"
scope = ["crates/cadetta-cli/src/cmd/ack.rs"]
+++

found while working ~G0D49J2: project_candidates in ack.rs returns no obligations because no source->IR->DigestGraph pipeline exists in the CLI yet; wire closure/deps/judgement per obligation and the author lookup (git log of covered nodes).
