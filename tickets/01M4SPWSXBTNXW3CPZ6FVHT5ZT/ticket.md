+++
id = "01M4SPWSXBTNXW3CPZ6FVHT5ZT"
title = "CLI skeleton, output formats and exit codes"
type = "story"
category = "todo"
priority = "medium"
points = 3
parent = "01M4SPWHHNRSGB9QZ04CMHWRRB"
reporter = "lognd"
created = "2026-10-13T04:03:55Z"
updated = "2026-10-13T04:04:03Z"
persona = "design engineer"
capability = "run cadetta with --format text|json, -v and --cwd"
outcome_text = "scripts and humans both get usable output"
labels = ["creates:crates/cadetta-cli/src/{main,output}.rs"]
scope = ["crates/cadetta-cli/src/{main,output}.rs"]

[[links]]
kind = "blocks"
target = "01M4SPWT0RZH7FJYEZ8J7MBN3Z"

[[acceptance]]
text = "Given diagnostics at or above fail_on, when a command finishes, then exit code is 1"
bound = false

[[acceptance]]
text = "Given a usage or I/O error, when a command finishes, then exit code is 2"
bound = false

[[acceptance]]
text = "Given stdout is not a terminal, when run, then json is the default format"
bound = false
+++
