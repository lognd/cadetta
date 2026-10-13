+++
id = "01M4STAQK1TBEA2M22FKW6EQ0D"
title = "Conformance fixtures for emitters/sw-equations"
type = "story"
category = "in-progress"
priority = "medium"
points = 5
parent = "01M4STAPYJBMRZBEY8HJH6F6XS"
reporter = "lognd"
created = "2026-10-13T05:03:58Z"
updated = "2026-10-13T17:21:55Z"
persona = "spec author"
capability = "verify an implementation against the spec"
idempotency_key = "replan13:Conformance fixtures for emitters/sw-equations"
labels = ["creates:tests/conformance/emitters/sw-equations/**"]
scope = ["tests/conformance/emitters/sw-equations/**"]

[[links]]
kind = "blocks"
target = "01M4STCF3A4JQGPYSE7BBHK4BD"

[[acceptance]]
text = "Given ambiguities found while writing fixtures, when found, then each is filed as a ticket against the spec (docs/spec/README.md)"
bound = true

[[acceptance]]
text = "Given docs/spec/emitters/sw-equations.md, when fixtures are written, then every normative rule of emitters/sw-equations has a must-fire and a must-not-fire fixture pair under tests/conformance/emitters/sw-equations/ (execution awaits runner discovery of nested docs, ~EB99AA4, and non-check entries, ~6BCVN48)"
bound = true
+++
