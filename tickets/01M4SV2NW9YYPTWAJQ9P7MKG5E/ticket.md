+++
id = "01M4SV2NW9YYPTWAJQ9P7MKG5E"
title = "Python solver decorator cadetta.solve.solver"
type = "story"
category = "todo"
priority = "medium"
points = 5
parent = "01M4SV2NDDVYW3WN79XN9C707C"
reporter = "lognd"
created = "2026-10-13T05:17:02Z"
updated = "2026-10-13T05:17:04Z"
persona = "solver author"
capability = "write a solver as a decorated Python function"
outcome_text = "I speak cadetta.solve/1 without protocol code"
labels = ["creates:bindings/python/python/cadetta/solve.py", "creates:bindings/python/tests/test_solve.py"]
scope = ["bindings/python/python/cadetta/solve.py", "bindings/python/tests/test_solve.py"]

[[links]]
kind = "blocked-by"
target = "01M4STAMZCNT7MPR1PGZ5GVZZ5"

[[links]]
kind = "blocked-by"
target = "01M4SV2NGV5P9KBBT7987BZCKQ"

[[links]]
kind = "supersedes"
target = "01M4STANJE3QCD30B9Z3G45QS8"

[[acceptance]]
text = "Given a decorated function, when .serve() reads a cadetta.solve/1 request on stdin, then it writes a valid response with the solver version on stdout (bindings.md 3.3, solvers.md)"
bound = false

[[acceptance]]
text = "Given an input or output whose kind differs from the declaration, when served, then a protocol error naming the output is returned (bindings.md 3.3)"
bound = false
+++

Supersedes the dropped story ~3G45QS8 (Python solver helper package, helpers/python/**); that ticket stays dropped and is not reopened.
