# Language bindings

One directory per language, each a thin wrapper over the Rust core
(`docs/spec/bindings.md`). A language without a directory here still
works through the JSON interfaces: `cadetta --format json`, the IR
schema, the solver protocol and the adapter protocol.

| Directory | Package | Registry |
|---|---|---|
| `python/` | `cadetta` (`import cadetta`, plus the `cadetta` command) | PyPI |
| `c/` | `cadetta-capi` (`libcadetta_capi`, `cadetta.h`) | release archives |

Adding a binding:

1. Create `bindings/<language>/` with its native crate (a workspace
   member through `bindings/*`; it stays out of `default-members`).
2. Add its package files, tests and a README. Test against the
   conformance corpus, not just smoke tests.
3. Add a CI job for it, and exclude its crate from the Rust jobs
   (`--exclude <crate>`).
4. Add a release job publishing it at the workspace version.
5. Add a row to `docs/spec/bindings.md` section 2 and to this table.
