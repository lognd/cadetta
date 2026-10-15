Fixed `cadetta check` to read `#[allow]` on `check` and `require` statements: an allow on a `check` now suppresses, and one on a `require` raises W003 while C001 stays an error.
