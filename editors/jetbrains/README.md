# Cadetta for JetBrains IDEs

JetBrains IDEs read TextMate bundles, so no separate plugin is needed.

1. Open Settings > Editor > TextMate Bundles.
2. Click `+` and select the `editors/vscode` directory of this repository
   (it contains `package.json` and `syntaxes/cadetta.tmLanguage.json`).
3. Apply. `.cdt` files now highlight.

## Language server (primary): LSP4IJ

Install the LSP4IJ plugin (Red Hat), then Settings > Languages & Frameworks >
Language Servers > `+` (New Language Server):

1. Name: `Cadetta`.
2. Command: `cadetta lsp` (use the full path if `cadetta` is not on the IDE's
   PATH).
3. Mappings > File name patterns: `*.cdt`, language id `cadetta`.

Diagnostics and formatting work; hover and code lens are not yet provided
(ticket ~0CXQ6EZ). Not verified in a JetBrains IDE.

## Fallback: External Tool

Without LSP4IJ, add an External Tool (Settings > Tools > External Tools)
running `cadetta check --format short` with working directory
`$ProjectFileDir$`, and an output filter regex
`$FILE_PATH$:$LINE$:$COLUMN$:` so the output is clickable.
