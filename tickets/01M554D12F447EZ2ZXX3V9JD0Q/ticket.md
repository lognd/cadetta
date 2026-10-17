+++
id = "01M554D12F447EZ2ZXX3V9JD0Q"
title = "sync file globs never match on Windows (backslash relative paths)"
type = "bug"
category = "in-progress"
priority = "medium"
points = 1
reporter = "lognd"
created = "2026-10-17T14:31:37Z"
updated = "2026-10-17T15:15:53Z"
scope = ["crates/cadetta-cli/src/cmd/sync.rs"]

[[acceptance]]
text = "A sync glob such as parts/*.SLDPRT matches files when relative paths use backslashes (normalised to forward slashes in both expand_files sites), with a unit test"
bound = true
+++

found while working ~NEHY9X7. expand_files in crates/cadetta-cli/src/cmd/sync.rs matches the glob against rel.to_string_lossy(), which has backslashes on Windows, so [[adapter]] files = ["parts/*.SLDPRT"] matches nothing and sync fails with 'adapter has no files to read'. Normalise rel to forward slashes before glob.matches (the same in the second site near line 1683). Workaround: list files without a wildcard.
