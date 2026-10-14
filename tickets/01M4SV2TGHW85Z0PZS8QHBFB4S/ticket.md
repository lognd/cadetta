+++
id = "01M4SV2TGHW85Z0PZS8QHBFB4S"
title = "SOLIDWORKS C# solution skeleton, initialize, features and CI build"
type = "story"
category = "in-progress"
priority = "medium"
points = 3
parent = "01M4SV2TCZM23R46EM75RYSA3B"
reporter = "lognd"
created = "2026-10-13T05:17:07Z"
updated = "2026-10-14T12:44:25Z"
persona = "design engineer"
capability = "connect to SOLIDWORKS through the adapter protocol"
labels = ["creates:adapters/solidworks/Cadetta.SolidWorks.sln", "creates:adapters/solidworks/Directory.Build.props", "creates:adapters/solidworks/src/cadetta-solidworks/**", "creates:adapters/solidworks/src/Cadetta.SolidWorks.Core/Protocol/**", "creates:adapters/solidworks/README.md", "creates:adapters/solidworks/**"]
scope = [".github/workflows/solidworks.yml", "adapters/solidworks/Cadetta.SolidWorks.sln", "adapters/solidworks/Directory.Build.props", "adapters/solidworks/src/cadetta-solidworks/**", "adapters/solidworks/src/Cadetta.SolidWorks.Core/Protocol/**", "adapters/solidworks/README.md", "adapters/solidworks/**"]

[[links]]
kind = "blocked-by"
target = "01M4SPWRGXS8XKJCF2H8FDKPKC"

[[links]]
kind = "blocked-by"
target = "01M4SPWRS0CDJT4GYC92AB1HCD"

[[acceptance]]
text = "Given the solution, when built, then it has the projects of solidworks.md 1 and only Cadetta.SolidWorks.Com and the add-in reference the interop assemblies (from SOLIDWORKS_API_DIR, never committed)"
bound = true

[[acceptance]]
text = "Given cadetta-solidworks, when initialize is sent, then name, version, capabilities read and drive, features frames, instances, create_frames, tolerances and rename_globals, native units, precision and platforms windows are returned, protocol types are generated from docs/schemas/adapter-protocol.json, and stdout carries protocol only (adapters.md 5; solidworks.md 1)"
bound = true

[[acceptance]]
text = "Given CI, when the solidworks workflow runs, then Core, the executable (against a fake session) and the tests build and pass on Linux and Windows without SOLIDWORKS"
bound = true
+++
