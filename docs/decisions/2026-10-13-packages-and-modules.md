# Packages, modules and visibility, Rust style

Status: current
Owner: cadetta
Decisions: none
Audience: contributors

- Date: 2026-10-13
- Ticket: none (pre-ledger)

## Context

Draft 1.4 had dotted namespaces declared by file headers, everything
public, and no reuse across projects (OQ-11). The owner asked for full
package, import and public module support like Rust's, and for no
public registry yet. Shared shop policies and parts libraries are the
reuse case: one shop's fastener rules and standard parts serve several
products, each in its own repository.

## Decision

1. **Packages are the unit of reuse.** A package is a `cadetta.toml`
   with `[package]`. Dependencies come from local paths or git repos,
   resolve to one version per package, and are pinned by content digest
   in `cadetta.lock`. `cadetta update` re-resolves them. Workspaces
   group packages under one lock. There is no registry yet (OQ-19).
2. **Modules are the file tree under `src`.** There are no `mod`
   declarations and no namespace headers. A canonical path is
   `package::module::Item`, so `std.fasteners::X` becomes
   `std::fasteners::X`.
3. **Items are private by default.** `pub` and `pub(package)` widen
   them, `pub use` re-exports, and using an item outside its visibility
   is an error. Component members are the opposite: public by default,
   because designs read their parts' values everywhere. `priv` on a
   member keeps it inside its package (closes OQ-20). It controls who
   may depend on the member, not who may see it: `why`, `impact` and
   the IR still show it. That keeps a library's intermediates out of its
   semver promise. There is no `protected`, since components do not
   inherit.
4. **`use` follows Rust.** Paths start at `package::`, `self::`,
   `super::` or a dependency's name, and globs, aliases and nested
   trees are allowed. A bare name resolves in a fixed order: members,
   then the module, explicit imports, globs, and finally the prelude.
5. **Dependencies feed the obligation graph.** A dependency's content
   digest enters the closures of everything that reads it, so a library
   update makes exactly the affected checks, sign-offs and syncs stale.
   A library's own attestations travel with it, and a dependent may
   trust named roles.

## Consequences

- Every example, golden case and path in the spec moved to the new
  form. `[project] namespace`, `sources` and `roots` are gone from
  `cadetta.toml`. New codes: E005 (reworded), E012-E017, U002.
- Semver for a library of parts is about values as much as names: a
  changed value that flips a dependent's verdict is a breaking change.
  The obligation graph reports it whatever the version says.

## Alternatives considered

- Keep namespaces and add packages on top: two naming systems for one
  job, and file headers that can disagree with the file's location.
- Public members by Rust's default (private): reading a part's values
  across an assembly is the point of the language, so making it opt-in
  would add `pub` to nearly every member.
- A registry in 0.1.0: no shared libraries exist outside one company
  yet, and git sources cover that case.
