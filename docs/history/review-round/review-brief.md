# Final review pass: brief (shared by all reviewers)

Repo: /home/logan/projects/cadetta. The product is a specification:
docs/spec/*.md (README, quantities, language, geometry, ir, adapters,
emitters/sw-equations, obligations, solvers, stdlib, bindings, lints,
config-cli) plus decisions in docs/decisions/. Read docs/spec/README.md
first, then everything your angle touches (read whole documents, not
excerpts; the cohesion question needs the full picture).

The owner's bar: "Everything should feel cohesive and inherently
useful/powerful." Your job is to find what feels underdeveloped,
half-integrated, inconsistent, or not worth its cost, from your angle.
The owner mainly uses SOLIDWORKS. The core is Rust; Python is the first
binding; JSON is the catch-all for every other language.

Rules:
- Do NOT edit any repo file. Write your findings to the output file
  named in your task, and nothing else.
- ASCII only. No emojis.
- Each finding: severity (blocker = the spec contradicts itself or a
  promised capability cannot work; gap = a capability a user will
  reasonably expect is missing or unspecified; polish = wording,
  naming, cross-reference), the doc and section, what is wrong in one or
  two sentences, and a concrete proposed fix (spec text or rule, not
  "clarify this").
- Prefer fewer, deeper findings over many shallow ones. At most 12
  findings, ordered by severity. Put a 3-line summary at the top: how
  developed your area feels (1-5) and the single most important fix.
- Do not propose features outside the stated 0.1.0 scope unless their
  absence makes an in-scope feature useless; then say so explicitly.
- Reply to the coordinator in at most 80 words: the output path, the
  counts per severity, and the top finding.
