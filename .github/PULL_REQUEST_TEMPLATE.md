## Ticket

<!-- The frob ticket handle, e.g. ~ABC1234, or "none -- small/obvious change" -->

## Summary

<!-- What does this change do, and why? -->

## Verification

- [ ] `frob check` is green
- [ ] Rust: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`
- [ ] Python adapter (in `adapters/freecad`): `uv run pytest`, `uv run ruff check`, `uv run ruff format --check`

## Checklist

- [ ] Behavior changes update `docs/spec` in the same change
- [ ] Pilots and the conformance corpus updated where behavior changed
- [ ] Changelog fragment added in `changelog.d/` for user-visible changes
- [ ] Commit messages follow `<type>(<scope>): <imperative summary>`
- [ ] ASCII only (no emoji, no smart quotes)
- [ ] AI assistance: none / <tool>, <how used>
- [ ] I understand and can explain every line of this change
