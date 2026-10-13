# Packages, modules and visibility

Cadetta code is organised the way Rust code is. A **package** is the
unit of reuse and versioning. Its **modules** come from its file tree,
and **visibility** decides what other modules and packages may use. A
shop's fastener policy, a library of standard parts and a product
design are each a package, and a product depends on the others by
version.

## 1. Packages

A package is a directory holding `cadetta.toml` with a `[package]`
table:

```toml
[package]
name = "acme.rover"          # lower.dotted (N001); the root of every canonical path
version = "0.3.0"            # semver
src = "src"                  # module root (default "src")

[dependencies]
"acme.shop"  = { path = "../shop", version = "1" }
"acme.parts" = { git = "https://example.com/acme/parts.git", tag = "v2.1.0" }
```

- A dependency names a package and a source: `path`, or `git` with
  `tag`, `rev` or `branch`. A registry source is OQ-19. `version` is a
  semver requirement the resolved package must meet (E012 otherwise).
- `std` is an implicit dependency at the compiler's own version, and no
  package may be named `std`.
- Resolution yields exactly one version of each package for the whole
  project (E013 on a conflict). There is no side-by-side duplication: a
  part has one definition, so two versions of one library would be two
  different parts with the same name.
- `cadetta.lock` pins every resolved dependency with a `package` record:
  name, version, source, and the digest of its content
  (obligations.md 3). `cadetta update [NAME]` re-resolves. A dependency
  whose content no longer matches its pinned digest is E014.
- A **workspace** is a directory whose `cadetta.toml` has `[workspace]
  members = ["rover", "crawler", "shop"]`. Members share one lock and
  one `build/`, and depend on each other by path. A product line with a
  shared shop package is one workspace.
- Loading rules (0.1.0): a dependency with neither `path` nor `git`
  (a bare version string or `{ version = ".." }`) is rejected as an
  unsupported registry source (OQ-19). A workspace member without a
  `cadetta.toml` is E005, and a member must be a package, not another
  workspace. The workspace lock is `<root>/cadetta.lock` and the build
  directory `<root>/build`. The implicit `std` dependency is versioned
  with the compiler.
- Resolution rules (0.1.0): the walk is breadth first from the root
  packages in name order, and the first requirement to reach a package
  selects its source and version. A first requirement the selection
  fails is E012. A later requirement it fails is E013, and no second
  copy is loaded. A later, different source for a selected package is
  ignored with a warning. A dependency without a `cadetta.toml`, a
  failed git checkout, or a workspace in place of a package is E005. A
  package that declares no `version` meets no `version` requirement.
  `version` accepts comma-joined `^`, `~`, `=`, `<`, `<=`, `>`, `>=`
  and wildcard comparators; a bare version is caret; pre-releases never
  match. Git and file access are injected, never made by the resolver.

## 2. Modules

The module tree is the file tree under `src`:

| File | Module |
|---|---|
| `src/lib.cdt` | the package root, `acme.rover` |
| `src/frame.cdt` | `acme.rover::frame` |
| `src/drive.cdt` | `acme.rover::drive` |
| `src/drive/motor.cdt` | `acme.rover::drive::motor` |

- Every `.cdt` file under `src` is a module; there are no `mod`
  declarations and no `namespace` headers. A directory `drive/` holds
  the children of `drive.cdt`, which may be absent when the parent
  module has no items of its own. Module names follow the case canon
  (`snake_case`, N001), and a file that breaks it is E015.
- Canonical paths are `package::module::...::Item`, then members
  (ir.md 2): `acme.rover::drive::motor::Gm42A.rated_torque`,
  `acme.rover::Rover/wheel[2].hub_bore`. Moving a file is a rename and
  is recorded with `was` on the moved items (5).
- Files outside `src` (bindings, solver scripts, cited documents) are
  referenced by path relative to the package directory.
- Loading rules: only `*.cdt` files are modules and other files are
  ignored. Every directory and file stem under `src` must be
  `snake_case` (E015). A directory without a parent file (`drive/`
  with no `drive.cdt`) still yields the implicit parent module. A
  missing or empty `src` is E005. Files are sorted by path before
  compile. A path that is absolute or escapes the package directory is
  rejected.

## 3. Visibility

- Items are private by default: visible in the module that declares
  them and in its descendants. `pub` makes an item visible everywhere
  the module is reachable. `pub(package)` limits that to the declaring
  package.
- An item may only be used where it is visible (E016). A `pub` item
  whose signature names a private item is E017. This covers an
  interface member's type, a component parameter's type, and a
  function's argument and result types.
- Members of a component (parameters, `let`s, fits, checks, parts) are
  visible wherever the component is, because a design reads its parts'
  values constantly. This is the opposite of Rust's default, on purpose.
- `priv` on a member restricts its use to the declaring package:
  `priv row = Iso4762Head[thread]`. It is for a library's
  intermediates, which its public members are computed from but which
  dependents should not build on. It controls dependency, not sight: a
  `priv` member is still evaluated, shown by `why` and `impact`, carried
  in the IR, and bound by the package's own bindings. Reading it from
  another package is E016. A member that implements an interface member
  cannot be `priv` (E017). There is no `protected`: components do not
  inherit from components, so there is nothing for it to mean.
- `std` exports exactly the items stdlib.md lists, all `pub`.
- A private item that nothing uses is U002 (warning), the analogue of
  Rust's dead-code lint. A private parameterized component with no
  instances is U002, not I004. I004 applies only to roots of the
  package being checked.

## 4. Use

```
use     = [ "pub" [ "(" "package" ")" ] ] "use" usepath [ "as" IDENT ]
        | [ "pub" ] "use" usepath "::" ( "*" | "{" usetree { "," usetree } "}" )
usepath = ( "package" | "self" | "super" { "::" "super" } | PKG ) { "::" IDENT }
usetree = IDENT [ "as" IDENT ] | IDENT "::" ( "*" | "{" usetree { "," usetree } "}" )
PKG     = IDENT { "." IDENT }                  // a dependency's dotted name, or std
```

- Paths start from the current package (`package::frame::Frame`), the
  current module (`self::`), a parent (`super::`), or a dependency by
  name (`acme.shop::policy::AcmeShop`, `std::fasteners::ScrewJoint`).
- `use path as alias` binds a module or item to a local name, and
  `use path::*` imports every visible item of a module.
- `pub use` re-exports, so a package presents a curated surface. A
  library can say `pub use self::fasteners::{ScrewJoint, MetricThread}`
  in `lib.cdt`, and users write `acme.shop::ScrewJoint`. A re-export is
  an alias, not a copy: the canonical path is still the definition's.
- A fully qualified path is legal anywhere without a `use`.
- Resolution order for a bare name:
  1. the component's own members and arguments;
  2. items of the current module;
  3. explicit `use` names;
  4. glob names;
  5. the prelude (language.md 4).

  Two globs that bring the same name are E004 only where the name is
  used. An unresolved name is E002.

## 5. Renames, moves and versions

- `was` takes a full canonical path, so a moved file, a renamed module,
  or an item moved into a library keeps its bindings, waivers and
  attestations (N004 on uses of the old path).
- A library changes its semver major version when it removes or renames
  a `pub` item or a non-`priv` member without `was`, changes the kind of
  either, or changes a value a dependent's verdict reads (for a library of parts, values are
  the interface). This rule is a convention for library authors:
  cadetta does not compare version numbers with changes. What it
  enforces is the obligation graph: `cadetta update` followed by
  `cadetta check` on a dependent reports every verdict an update flips
  and every sign-off it makes stale, whatever the version number says.

## 6. Obligations across packages

- A dependency's content digest is part of the closure digest of
  everything that reads it, so updating a library makes exactly the
  dependents' checks, sign-offs and CAD syncs that read changed values
  stale.
- A package's lock may carry attestations of its own nodes, and they
  travel with the package. A dependent sees them as "attested upstream"
  in `cadetta obligations`. `[dependencies."acme.shop"] trust =
  ["source", "tolerance_review"]` lets those roles discharge the
  dependent's matching obligations on the dependency's nodes. Without
  `trust`, they are information only.

## 7. Diagnostics

| Code | Name | Default | Meaning |
|---|---|---|---|
| E012 | version-unsatisfied | error, fixed | A resolved dependency does not meet the `version` requirement. |
| E013 | version-conflict | error, fixed | Two requirements on one package cannot be met by a single version. |
| E014 | dependency-changed | error, fixed | A dependency's content differs from the digest pinned in the lock; run `cadetta update`. |
| E015 | bad-module-name | error, fixed | A file or directory under `src` is not a valid module name. |
| E016 | private-item | error, fixed | An item is used outside its visibility, or a `priv` member is read from another package. |
| E017 | private-in-public | error, fixed | A `pub` item's signature names a less visible item, or a member implementing an interface member is `priv`. |
| U002 | unused-private-item | warning | A private item is used nowhere in its package. |

## Open questions

- OQ-19: A package registry (publish, search, yank) versus git and path
  sources only. Recommendation: git and path for 0.1.0; a registry once
  shared parts libraries exist outside one company.
- OQ-20 (closed 2026-10-13, section 3): member privacy is `priv`,
  package-scoped, opt-in; members stay public by default.
