+++
id = "01M4WE56PSQJHJRG3BHYA5SB8A"
title = "Wire Project._run/_model to result.report_from_exit"
type = "task"
category = "in-progress"
priority = "medium"
points = 2
reporter = "Claude"
created = "2026-10-14T05:28:57Z"
updated = "2026-10-14T05:51:59Z"
labels = ["creates:bindings/python/tests/test_project_exit.py"]
scope = ["bindings/python/python/cadetta/project.py", "bindings/python/tests/test_project_exit.py"]

[[acceptance]]
text = "Given a design with error diagnostics, when Project.check runs, then Ok(report) is returned via result.report_from_exit"
bound = false
+++

Follow-up from ~58R6ZY3: report_from_exit exists in result.py but project.py still uses its own exit handling. Switch Project._run and Project._model over (error factory avoids a circular import).
