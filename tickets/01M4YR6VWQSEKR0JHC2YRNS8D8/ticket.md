+++
id = "01M4YR6VWQSEKR0JHC2YRNS8D8"
title = "language: E016 on expression-level references and priv member reads (check_use not called)"
type = "bug"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-15T03:03:06Z"
updated = "2026-10-15T03:03:06Z"
scope = ["crates/cadetta-sema/**"]
+++

found while working ~ZDNTT6T: only use targets (check_uses) and E017 signatures are wired; VisibilityChecker::check_use for expression paths and priv member chains is still uncalled.
