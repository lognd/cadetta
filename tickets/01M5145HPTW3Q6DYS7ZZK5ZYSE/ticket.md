+++
id = "01M5145HPTW3Q6DYS7ZZK5ZYSE"
title = "Project.open(workspace root).evaluate() succeeds; bindings.md 3.2 says usage error"
type = "bug"
category = "in-progress"
priority = "medium"
points = 1
reporter = "lognd"
created = "2026-10-16T01:10:33Z"
updated = "2026-10-16T01:24:28Z"
scope = ["bindings/python/python/cadetta/project.py"]

[[acceptance]]
text = "Workspace-root evaluate() behaviour and spec agree; KNOWN_DIVERGENCES entries removed"
bound = false
+++

Found while working CDRJTN9. Fixtures workspace-root-has-no-package/must-fire and evaluate-validates-ir/must-not-fire expect evaluate() at a workspace root to be the CLI usage error (exit 2); the binding returns Ok. Either fix the binding or amend docs/spec/bindings.md 3.2 and the fixtures. Replay marks both xfail in bindings/python/tests/test_binding_fixtures.py KNOWN_DIVERGENCES.
