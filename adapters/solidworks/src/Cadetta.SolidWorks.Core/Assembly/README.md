# Assembly read

`read` of a `.SLDASM` reports, besides the observed IR (adapters.md 3):

- Component-scoped dimensions. Each loaded component's dimensions are
  appended with locator `Dn@Feature@File.SLDPRT` and a `component` field
  holding the in-assembly instance name `Name<N>` (SOLIDWORKS `Name2`
  `Bracket-1`, or `Sub-1/Bracket-1` in a subassembly, becomes `Bracket<1>`).
- `components`: one `{file, locator, component_file}` fact per instance,
  `component_file` being the file the instance actually holds. The host
  compares it with the slot primary's file; lint R011 (component-mismatch,
  warning) fires when they differ (lints.md 3 R, adapters.md 3.3).

Code: `ComponentNaming` (name forms), `ComponentFacts` (fact list, Core, no
COM), `Com/Assembly/SwComComponentReader` (component dimension walk).
Verified on SOLIDWORKS 2025 with a fixture assembly: the fact
`Fix<1> -> Fix.SLDPRT` and the `component` field on both of its dimensions.
