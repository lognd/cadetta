+++
id = "01M4ZG85CVMQ8CQ6NXHSTGZ0R5"
title = "Dependency check digest differs between its own universe and the dependent's"
type = "bug"
category = "todo"
priority = "medium"
points = 2
reporter = "lognd"
created = "2026-10-15T10:03:14Z"
updated = "2026-10-15T10:40:18Z"
scope = ["crates/cadetta-cli/src/universe.rs"]
+++

found while working ~Q06ZV6Q: a dependency's own-lock digest for check:acme.axle::Axle.bore_ok (ack inside the dependency) differs from the closure digest the dependent computes for the same node, so trusted upstream roles cannot discharge in practice. Make the digest context independent.
