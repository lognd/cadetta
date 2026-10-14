+++
id = "01M4SV2P5W45J454JP96ZMPC6J"
title = "Python binding test suite replays conformance corpus and asserts CLI parity"
type = "story"
category = "in-progress"
priority = "medium"
points = 8
parent = "01M4SV2NDDVYW3WN79XN9C707C"
reporter = "lognd"
created = "2026-10-13T05:17:03Z"
updated = "2026-10-14T13:17:21Z"
persona = "maintainer"
capability = "prove the binding adds no semantics"
outcome_text = "binding and CLI cannot disagree"
labels = ["creates:bindings/python/tests/test_conformance.py"]
scope = ["bindings/python/tests/test_conformance.py"]

[[links]]
kind = "blocked-by"
target = "01M4SV2NMPSM42YZT9MXT4YVMW"

[[links]]
kind = "blocked-by"
target = "01M4SV2NRTBH99MYAY6EWBDP66"

[[links]]
kind = "blocked-by"
target = "01M4SV2NW9YYPTWAJQ9P7MKG5E"

[[links]]
kind = "blocked-by"
target = "01M4SV2NZT32CAT33D13T86646"

[[links]]
kind = "blocked-by"
target = "01M4SV2R23GPE64N4CZVNWRJHC"

[[acceptance]]
text = "Given every fixture in tests/conformance, when replayed through the binding, then its results equal the CLI --format json output (bindings.md 1)"
bound = false

[[acceptance]]
text = "Given the bindings CI job, when it runs, then the parity suite executes and gates the PR (bindings.md 2)"
bound = false

[[acceptance]]
text = "Given the conformance corpus, when replayed, then binding results equal the CLI JSON, including geometry, json and csv fixtures (bindings.md 1)"
bound = false
+++
