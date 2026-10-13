+++
id = "01M4ST8F1W8T0BKARAMG0D49J2"
title = "cadetta ack"
type = "story"
category = "in-progress"
priority = "medium"
points = 3
parent = "01M4ST8EJWTGQM9CMVBJVBATJT"
reporter = "lognd"
created = "2026-10-13T05:02:43Z"
updated = "2026-10-13T08:51:39Z"
persona = "design engineer"
capability = "attest a selection at current digests"
idempotency_key = "replan13:cadetta ack"
labels = ["creates:crates/cadetta-cli/src/cmd/ack.rs"]
scope = ["crates/cadetta-cli/src/cmd/ack.rs", "crates/cadetta-cli/src/cmd/mod.rs", "crates/cadetta-cli/src/lib.rs", "docs/spec/config-cli.md", "crates/cadetta-cli/src/env.rs", "design/model.grmb"]

[[acceptance]]
text = 'Given `cadetta ack SELECTOR --role R --reason "..."`, when run, then the lock gains an attestation at current closure digests (obligations.md 5)'
bound = true

[[acceptance]]
text = "Given a selector matching no node or a role not declared, when run, then exit 2 with a message (config-cli.md 2)"
bound = true

[[acceptance]]
text = "Given a repeat with no digest change, when run, then no lock change is made (obligations.md 3)"
bound = true

[[acceptance]]
text = "Given ack waiver:GLOB, --as EMAIL or --baseline, when run, then records follow obligations.md 4 and the reason baseline is used (config-cli.md 3)"
bound = true
+++
