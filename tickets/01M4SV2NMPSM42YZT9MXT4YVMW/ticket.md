+++
id = "01M4SV2NMPSM42YZT9MXT4YVMW"
title = "Python Project.open, evaluate and design indexing with schema-generated IR models"
type = "story"
category = "todo"
priority = "medium"
points = 8
parent = "01M4SV2NDDVYW3WN79XN9C707C"
reporter = "lognd"
created = "2026-10-13T05:17:02Z"
updated = "2026-10-13T05:45:12Z"
persona = "Python user"
capability = "open a project and read the evaluated design"
outcome_text = "I can script against values without parsing JSON"
labels = ["creates:bindings/python/python/cadetta/project.py", "creates:bindings/python/python/cadetta/models/**", "creates:bindings/python/tests/test_project.py"]
scope = ["bindings/python/python/cadetta/project.py", "bindings/python/python/cadetta/models/**", "bindings/python/tests/test_project.py"]

[[links]]
kind = "blocked-by"
target = "01M4SPWQANZ48Y8HG2X84RNK2D"

[[links]]
kind = "blocked-by"
target = "01M4SPWQH44JWFXZDNTGEBMMNX"

[[links]]
kind = "blocked-by"
target = "01M4SPWSXBTNXW3CPZ6FVHT5ZT"

[[links]]
kind = "blocked-by"
target = "01M4SV2NGV5P9KBBT7987BZCKQ"

[[acceptance]]
text = "Given a project path, when Project.open then evaluate run, then a typani Result wraps the design IR as pydantic models generated from the IR JSON schema (bindings.md 3.1, 3.2)"
bound = false

[[acceptance]]
text = 'Given design["acme.rover::Axle.end_diameter"], when nominal, max, mid, kind and citation are read, then they match the CLI ir JSON (bindings.md 3.1)'
bound = false

[[acceptance]]
text = "Given an unknown path, when indexed, then a Result error is returned, not a KeyError (bindings.md 3)"
bound = false

[[acceptance]]
text = "Given Project.open(path=None, config=None), when called, then root discovery follows config-cli.md 1.7 and project.sources lists the resolved sources (bindings.md 3.2)"
bound = false
+++
