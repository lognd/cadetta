+++
id = "01M55EASQHS5H5WW32KQK2F312"
title = "SOLIDWORKS: Link of a count dimension creates the global as 4000mm (scalar scaled as length)"
type = "bug"
category = "in-progress"
priority = "medium"
points = 2
reporter = "lognd"
created = "2026-10-17T17:25:10Z"
updated = "2026-10-17T17:28:06Z"
scope = ["adapters/solidworks/src/**", "adapters/solidworks/tests/**"]
+++

found while working ~W5MBBTB: add-in Link on D1@LPattern1 logged 'global add ... = 4000mm'. Global create scales an integer dimension by the length factor; use DimensionUnits kind. Scope adapters/solidworks/src/**
