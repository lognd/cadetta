# Security Policy

## Supported versions

cadetta is pre-1.0 and the specification is still a draft; only the
latest published release receives security fixes.

| Version        | Supported |
|----------------|-----------|
| latest release | yes       |
| anything older | no        |

Upgrade before filing a report if you are not on the latest release.

## Reporting a vulnerability

Please do not file a public GitHub issue for a suspected vulnerability.

Use one of the following, in order of preference:

1. GitHub private vulnerability reporting: open the "Security" tab on
   [lognd/cadetta](https://github.com/lognd/cadetta), then "Report a
   vulnerability". This creates a private advisory thread that only the
   maintainer and you can see.
2. Email logan@logand.app if you cannot use GitHub's reporting flow.

### What to include

- A minimal reproduction: the smallest cadetta source file (or project
  tree) that triggers the issue, and the command run.
- The cadetta version (`cadetta --version`), operating system, and, for
  adapter issues, the CAD application and version.
- What you expected versus what happened, and why you believe it is a
  security issue rather than a correctness bug.

### Response expectations

The maintainer aims to acknowledge a report within 7 days. There is no
bounty program. A fix timeline depends on severity and will be discussed
with you in the private advisory thread.

## Scope notes specific to this project

cadetta is a local parameter language and linter for 3D CAD. It reads
source files, resolves units, fits and constraints, and exchanges data
with CAD applications through external adapters. Most legitimate reports
fall into one of these areas:

- **Parsing untrusted input.** The `cadetta` CLI parses source files,
  project manifests and `cadetta.ir/1` documents. Parsing must never
  execute, import or evaluate their content. A crafted input that
  crashes the parser is a bug; one that achieves code execution is high
  severity.
- **External adapters.** The CLI launches adapters (for example
  `adapters/freecad`) as subprocesses and exchanges IR with them. A
  report of argument or environment injection into an adapter launch, or
  of an adapter reply that makes the host misbehave, is in scope.
  Running an adapter against a CAD application executes that
  application's own scripting, so do not run cadetta against untrusted
  models without understanding that.
- **File writes escaping the project.** A crafted path or symlink that
  makes cadetta write or delete outside the project root is in scope and
  high severity.
- **Supply chain.** Releases publish to crates.io and PyPI through
  trusted publishing from protected environments. A report about the
  release workflow or its pinned actions is in scope.

If you are unsure whether something is in scope, report it anyway and
let the maintainer make the call -- a false positive costs little, a
missed report costs a lot.
