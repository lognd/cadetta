+++
id = "01M4V13T5ZKFSRJAZ310ZMVF75"
title = "language.md ambiguities found writing conformance fixtures"
type = "bug"
category = "todo"
priority = "medium"
points = 2
reporter = "lognd"
created = "2026-10-13T16:21:45Z"
updated = "2026-10-16T07:10:18Z"
scope = ["docs/spec/language.md"]
+++

found while working ~7MEVQAH. Fixtures under tests/conformance/language/ are spec-true; these sides disagree with the binary:  T001 'matches none of its rows' needs definition (literal equals no key and no column cell of rows carrying the citation); G001 vs decisions paragraph conflict (untyped points allow array+point per element, typed Point2 on D does not); part-count/index/roll-up/array-literal errors have no code (assume EVAL, fractional count Q009); U002 vs unused private parameterless root unclear; severity-split cases use non-code rule names in expect.json.
