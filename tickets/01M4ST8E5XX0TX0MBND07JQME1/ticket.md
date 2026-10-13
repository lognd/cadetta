+++
id = "01M4ST8E5XX0TX0MBND07JQME1"
title = "Interfaces, alternates and variant evaluation (cadetta-variants)"
type = "epic"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T05:02:42Z"
updated = "2026-10-13T05:05:11Z"
idempotency_key = "replan13:Interfaces, alternates and variant evaluation (cadetta-variants)"
labels = ["creates:crates/cadetta-variants/**"]
scope = ["crates/cadetta-variants/**"]

[[links]]
kind = "blocks"
target = "01M4STANRAWHV180C7RH0NQAY9"
+++

Outcome: swapping a part is one edit and every approved alternate is verified continuously by slot-at-a-time variant evaluation, per docs/spec/language.md 3.2.

Success metric: the language.md section 1 Caster example reports viability of Colson2Series, with linear cost in alternates.
