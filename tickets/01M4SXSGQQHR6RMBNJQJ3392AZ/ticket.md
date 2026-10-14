+++
id = "01M4SXSGQQHR6RMBNJQJ3392AZ"
title = "Golden corpus runner"
type = "story"
category = "in-progress"
priority = "medium"
points = 8
parent = "01M4STAPYJBMRZBEY8HJH6F6XS"
reporter = "lognd"
created = "2026-10-13T06:04:28Z"
updated = "2026-10-14T02:56:59Z"
persona = "maintainer"
capability = "check each golden design against its hand-evaluated expectations"
labels = ["creates:crates/cadetta-cli/tests/golden/**"]
scope = ["crates/cadetta-cli/tests/golden/**"]

[[links]]
kind = "blocked-by"
target = "01M4SPWT3FP4SN4RTCHE631Y52"

[[links]]
kind = "blocked-by"
target = "01M4SPWTA1PQV15S63741V30XT"

[[links]]
kind = "blocked-by"
target = "01M4STAQ2HJ1561N1M6VJHY70G"

[[acceptance]]
text = "Given each tests/golden/<NN-name>/ case, when the runner executes, then it runs `cadetta check --format json` and every [[artifact]] command at the case root (tests/golden/README.md Running)"
bound = false

[[acceptance]]
text = "Given [[value]] entries, when compared, then quantities are compared as exact quantities in any unit, at instance paths, honouring optional min, max and variant (tests/golden/README.md expected.toml)"
bound = false

[[acceptance]]
text = "Given [[verdict]] entries, when compared, then result (holds|violates|undecided) and conservative match per path and variant (tests/golden/README.md expected.toml)"
bound = false

[[acceptance]]
text = "Given [[diagnostic]] entries, when compared, then the list is closed for errors and warnings (an unlisted one is a failure) and info diagnostics are compared only when listed (tests/golden/README.md expected.toml)"
bound = false

[[acceptance]]
text = "Given [[artifact]] entries, when compared, then the produced file equals the expected file byte for byte after every `mask` regular expression match is replaced by `<masked>` in both (tests/golden/README.md expected.toml)"
bound = false

[[acceptance]]
text = 'Given status = "hand-evaluated", when the implementation disagrees, then the failure says whether the implementation or the hand evaluation is suspect; a case with status = "confirmed" fails as a regression (tests/golden/README.md)'
bound = false

[[acceptance]]
text = 'Given a case whose files carry `blake3:TBD` and the run first reproduces it, when confirmed, then the runner writes the real digests and sets status = "confirmed" (tests/golden/README.md Things hand evaluation cannot produce)'
bound = false

[[acceptance]]
text = "Given a case whose spec version is older than the current draft, when run, then it fails until ported (tests/golden/README.md Running)"
bound = false
+++
