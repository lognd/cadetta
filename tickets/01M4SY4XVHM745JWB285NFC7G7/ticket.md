+++
id = "01M4SY4XVHM745JWB285NFC7G7"
title = "cadetta-log crate: the single tracing subscriber owner (mirrors gob-log)"
type = "story"
category = "in-progress"
priority = "medium"
points = 5
parent = "01M4SPWFRR9DAEM60DX796GVPN"
reporter = "lognd"
created = "2026-10-13T06:10:42Z"
updated = "2026-10-13T06:17:41Z"
labels = ["creates:crates/cadetta-log/**"]
scope = ["crates/cadetta-cli/src/main.rs", "crates/cadetta-cli/src/lib.rs", "crates/cadetta-cli/src/logging.rs", "crates/cadetta-cli/Cargo.toml", "Cargo.toml", "frob.toml", "design/model.grmb", "crates/cadetta-log/**", "bindings/python/Cargo.toml", "bindings/python/src/lib.rs", "docs/**"]

[[acceptance]]
text = "Given CADETTA_LOG is set to a valid or invalid env-filter, when the CLI starts, then a valid filter is honoured and an invalid one is a usage error, not a panic (config-cli.md; mirrors gob-log init)"
bound = false

[[acceptance]]
text = "Given -v, -vv or -vvv, when CADETTA_LOG is unset, then the default level is info, debug or trace (warn at 0), always to stderr"
bound = false

[[acceptance]]
text = "Given a second init call, when it runs, then it returns an error and does not panic"
bound = false

[[acceptance]]
text = "Given any crate outside cadetta-log, when frob check runs, then a policy rule fails any tracing_subscriber use or print/eprint macro there (one observability story)"
bound = false
+++

Mirror ~/projects/frob-v2/crates/gob-log (init.rs: LOG_ENV, default_directive, build_filter, InitError, text/JSON layers, FmtSpan::CLOSE timing, ansi only on a terminal; redact.rs for secret masking). Replace the ad-hoc init_logging in crates/cadetta-cli/src/main.rs. Keep stdout for command output; logs only on stderr.
