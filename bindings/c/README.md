# cadetta C ABI

`cadetta-capi`: a stable `extern "C"` surface over the cadetta core, the
base for C, C++, MATLAB (MEX) and Julia (docs/spec/bindings.md 4).

Build: `cargo build -p cadetta-capi` produces `libcadetta_capi.{so,dylib,dll}`
(cdylib) and `libcadetta_capi.a`; the header is `include/cadetta.h`.

## Surface

| Symbol | Purpose |
|---|---|
| `cadetta_version()` | core version string (static) |
| `cadetta_run(argc, argv, &out, &err)` | run the CLI in-process; returns its exit code, stdout and stderr as heap strings |
| `cadetta_free(s)` | release a string from `cadetta_run` |

Design: one entry point, not one function per query. `cadetta_run` calls the
same `run_with_logging` as the CLI and the Python binding, so
`cadetta_run(--format json ...)` returns the CLI's JSON document by
construction (no second implementation to drift), and every present and
future command is reachable. Per-query typed structs are deliberately not
offered: C callers parse the JSON documents whose schemas live in
`docs/schemas/`. Errors: exit 0/1 mean a document is in `out`; 2 means
usage or I/O (message in `err`); `CADETTA_E_ARGS` (-1) and `CADETTA_E_PANIC`
(-2) are ABI-level failures. No call aborts the host process.

## Test

```sh
cargo build -p cadetta-capi -p cadetta
cargo test -p cadetta-capi
```

`tests/conformance_parity.rs` compiles `tests/parity.c` against the header
and library, replays the corpus `check`/`config`/`eval` source fixtures
through it and through the CLI binary, and asserts equal exit code and
`--format json` stdout.
