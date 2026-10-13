+++
id = "01M4SPWSXBTNXW3CPZ6FVHT5ZT"
title = "CLI skeleton, output formats and exit codes"
type = "story"
category = "in-progress"
priority = "medium"
points = 3
parent = "01M4SPWHHNRSGB9QZ04CMHWRRB"
reporter = "lognd"
created = "2026-10-13T04:03:55Z"
updated = "2026-10-13T06:34:22Z"
persona = "design engineer"
capability = "run cadetta with --format text|json, -v and --cwd"
outcome_text = "scripts and humans both get usable output"
labels = ["creates:crates/cadetta-cli/src/{main,output}.rs"]
scope = ["crates/cadetta-cli/src/{main,output}.rs", "crates/cadetta-cli/src/lib.rs", "crates/cadetta-cli/Cargo.toml", "docs/spec/config-cli.md"]

[[links]]
kind = "blocks"
target = "01M4SPWT0RZH7FJYEZ8J7MBN3Z"

[[links]]
kind = "blocks"
target = "01M4SPWT3FP4SN4RTCHE631Y52"

[[links]]
kind = "blocks"
target = "01M4SPWT6N1HY3MHN544GZJYS9"

[[links]]
kind = "blocks"
target = "01M4SPWTA1PQV15S63741V30XT"

[[acceptance]]
text = "Given diagnostics at or above fail_on, when a command finishes, then exit code is 1"
bound = false

[[acceptance]]
text = "Given a usage or I/O error, when a command finishes, then exit code is 2"
bound = false

[[acceptance]]
text = "Given stdout is not a terminal, when run, then json is the default format"
bound = false

[[acceptance]]
text = "Given --format text|short|github|sarif|json or CADETTA_FORMAT, --cwd and --config, when run, then they behave as config-cli.md 4, 5; text is always the default"
bound = false
+++
