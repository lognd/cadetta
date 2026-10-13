# stdlib conformance fixtures

Pairs for every rule and diagnostic in `docs/spec/stdlib.md`. Entries
are `check` (runs today) or `eval` (value fixtures; the runner cannot
execute non-check entries yet, ticket ~6BCVN48). Failing `check`
fixtures are listed in `../known-gaps.toml` against ~N8G7AVG, ~6K58FR1
and ~PGR0M75. The `engaged` joint check has no must-fire fixture (see
stdlib.md, Conformance decisions).
