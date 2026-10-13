# Cadetta for JetBrains IDEs

JetBrains IDEs read TextMate bundles, so no separate plugin is needed.

1. Open Settings > Editor > TextMate Bundles.
2. Click `+` and select the `editors/vscode` directory of this repository
   (it contains `package.json` and `syntaxes/cadetta.tmLanguage.json`).
3. Apply. `.cdt` files now highlight.

To see diagnostics, add an External Tool (Settings > Tools > External Tools)
running `cadetta check --format short` with working directory
`$ProjectFileDir$`, and an output filter regex
`$FILE_PATH$:$LINE$:$COLUMN$:` so the output is clickable.
