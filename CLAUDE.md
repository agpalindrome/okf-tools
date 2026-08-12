# okf-tools — repo guide for Claude

A Cargo workspace holding three crates: **`okf-graph`** and **`okf-normative`**,
two general-purpose tools for validating an [OKF][okf] Knowledge Bundle, and
**`deon`**, the archived reference implementation they supersede.

The architecture — how these crates relate to `pacioli`, to the Knowledge
Bundles, and to the applied accounting agent — lives in the README of the
[`okf-stack` project board][board]. Read it before making structural decisions.
It is a working reference, not a settled spec, and it is edited as work
proceeds; per-repo authoritative documentation replaces it as each repo is
built out.

[okf]: https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md
[board]: https://github.com/orgs/ojhermann-org/projects/8

## The crates, and the boundary between them

- **`okf-graph` — topology and identity.** What Concepts exist, what they are
  called, what points at what, what resolves and what dangles. Node bodies are
  carried as **opaque payloads** and never interpreted here.
- **`okf-normative` — semantics.** The normative reading: the mechanical /
  judgment / election cut, grounding, coverage, conflict, defeat, termination
  at the seam. Consumes a validated graph; never re-parses a Bundle.
- **`deon` — archived.** Kept as reference, `publish = false`. See below.

**`okf-normative` depends on `okf-graph`, never the reverse.** Inside a
workspace it costs nothing to violate that, so it has to be deliberate: no
reaching around through dev-dependencies, and `okf-graph`'s tests must not
mention a color, a norm, or anything normative. The falsifiable version: **if
`okf-graph`'s test suite cannot be read by someone who has never heard of
deontic logic, the boundary is not real.**

The split exists so the halves are independently testable — normative rules
against in-memory graphs, with no files and no OKF — and so assumptions about
an unsettled format stay quarantined in one crate. It is **not** a plugin
architecture: there is one evaluation today, and no second one can yet be
named.

## Order of precedence

**okf > `okf-graph` > `okf-normative` > applications.** The spec is upstream of
everything here: we match its structure and adapt as it changes, rather than
bending it to what is already built. Inside the repo, topology is upstream of
semantics, and both are upstream of anything built on them.

- **`deon` is reference, not oracle.** It implements an earlier reading; where
  it and the spec disagree, the spec wins and `deon` stays as it is.
- **A spec revision is ordinary work.** OKF is **v0.2** as of 2026-07-24 (§13
  lists the changes: `timestamp` → `generated.at`, `# Citations` → `sources`,
  plus the new §5 and §10 families). Read the "Changes from" section, fix what
  went stale, file what is newly modellable.

## Scope: the tools, not the knowledge

Neither tool is accounting-specific, and neither is an authority on any domain.
Norm *content* lives in Knowledge Bundles elsewhere — one per authority (GAAP,
IFRS, …), authored and reviewed by domain experts. `crates/deon/examples/`
holds three seed norms as **fixtures**, not as a catalog to grow.

Two limits follow, and both matter more than they look:

- **Fidelity is not machine-checkable.** Nothing here verifies that a Bundle
  faithfully represents the standard it claims to encode. Structural validation
  plus a machine author yields a Bundle that is well-formed, fully cited,
  internally consistent, and wrong. Expert review is the gate; say so rather
  than implying otherwise.
- **Declaration, not derivation.** The checker verifies an author was
  internally *consistent* about the seam. It cannot verify that a predicate
  declared `mechanical` deserves the colour — that needs domain knowledge these
  tools deliberately do not hold.

## The core discipline

Every predicate is **colored** — `mechanical` (decidable from seam data),
`judgment` (open-textured, must cite where it grounds), or `election` (a
discretionary entity choice, grounding in entity policy). The invariant, stated
once: **anything that could fall on either side of the seam must say which**,
and silence is a defect rather than a default.

`okf-normative` inherits two commitments from `deon`:

- **A ledger with a checker, not an inference engine.** Normative input comes
  from expert users; agents assist, they do not decide. Whether something is
  mechanical is a standard-setter's policy choice, not a discoverable property.
- **Deriving is not inferring.** Computing an interior node's colour from
  expert-declared leaves is arithmetic over declarations, and is on the table —
  it is what would let the checker *catch* a mis-colouring rather than merely
  notice two declarations disagreeing.

## The archived `deon` crate

It stays a **live workspace member** rather than an excluded directory: its
fixtures and 27 tests are a behavioural specification worth running as a
baseline while the new crates are built. An excluded crate rots; a green one
stays useful.

- Keep it compiling and green. Do **not** extend it, add checks to it, or build
  new work on it.
- Do **not** delete it or its fixtures without asking.
- Its **docs are not archived with it.** `docs/DESIGN.md`, the two
  `docs/spikes/` notes, and `docs/probes/` are the intellectual record the new
  crates inherit — the seam, the colouring model, N1, F5, the grounding
  discipline. Only the implementation is superseded.

## Build and verify

- **Run it:** `nix run . -- crates/deon/examples/` for the always-on checks;
  add `--okf <bundle>` for the two needing a concept bundle (GROUND-3 anchor
  resolution, and coverage, which reads each subject's declared state space).
  The repo ships usable bundles under `crates/deon/tests/fixtures/`.
- **Test:** `cargo test --workspace`. Every check ships a green case (the
  seeds) and a red fixture, because a checker you have only seen say "clean" is
  not a checker. The `near-miss` and `claim-shapes` fixtures cover the
  *near-miss* forms specifically — this codebase's characteristic bug is a
  shape that parses, passes, and is silently never examined.
- **CI:** `nix flake check` is the single required status check on `main`
  (`.github/rulesets/main.json`, reconciled by `scripts/settings.sh`). It gates
  `cargo fmt --check`, `clippy -D warnings`, and the suite.
- **Repo settings as code:** `scripts/settings.sh --check` diffs both the
  ruleset and the About block (`.github/settings/about.json` — description,
  homepage, topics) against live GitHub; `--apply` writes them. It stays
  owner-run rather than wired into CI, so settings never change silently.
- **Git hooks:** entering the dev shell installs them, and **`prek` is the
  driver** (`package` in `flake.nix`, which argues the choice). The hook it
  generates guards the store path it pins and falls back to `PATH`, so a
  garbage-collection degrades it into a message naming `prek` rather than
  breaking `git commit` opaquely (#98). `checks.hook-fallback` installs a real
  hook and takes that path away, so a prek release dropping the guard fails CI
  instead of surfacing at someone's next collection. A hook that reports prek
  is gone, or a `.pre-commit-config.yaml` symlink gone stale against
  `flake.nix` — the way `nix flake check` and `git commit` come to disagree on
  one lint — are both fixed by re-entering the shell (`nix develop -c true`, or
  `direnv reload`). `--no-verify` is not the answer to either: it skips every
  check.
- **Docs lint at 80 columns**, and emphasis style must be *consistent within
  each file* (MD049 infers it from the first use — `docs/DESIGN.md` is
  underscore, this file is asterisk). Both bite often.

## Comments

Comments enhance the code; they do not narrate it. This repo argues for its
designs in prose, which is easy to mistake for licence to write a lot — it is
not. Say what has to be said, as succinctly as it can be said.

- **Keep the reasoning a reader cannot recover** from the code or the spec: why
  a type omits a field, why a defective input must still parse, why two
  different inputs deliberately give the same answer, what was rejected.
- **Cut restatement** of the spec, narration of what the code plainly does, and
  the rhetorical tail on a point already made.
- **Put a rationale on the item whose behaviour it explains**, not in the module
  header. One line for an obvious accessor or test; a paragraph only where a
  decision was made.

## Landing changes

- `main` is **PR-only** and branch-protected (required owner review + merge
  queue). `scripts/merge.sh <pr>` lands an owner-authored PR via ruleset
  bypass — the agent runs it **only on the owner's explicit ask for that PR**.
  Asking for a task is not an ask to merge the PRs the task produces; the
  required review is exactly the gate the bypass removes. This applies to any
  sub-agent you brief, which will act on whatever authority your brief claims
  for it.
- `merge.sh` gates on green CI, so no `--force` is needed; **`--force` stays
  owner-run/confirm-first.**

## Deletion & creation

- Do **not** weaken the colouring discipline, and do not let a rule's
  *recognition* depend on the thing that rule checks for — an aggregation that
  stops being an aggregation when you delete its citation is how a checker gets
  weaker exactly as its input gets more dishonest.
- Do **not** add an execution engine, a bespoke syntax, or neural components.
  Evaluation belongs to the applied project; that is a layer boundary now, not
  a deferral.
- Treat `docs/` as the load-bearing record of *why* things are shaped as they
  are, including what was rejected. Don't casually rewrite it.
- Where the OKF format fights us, **log the concrete case when it happens** — an
  eventual conversation with that community should rest on a dated list of real
  friction, not recollection. Taking anything upstream is the owner's call.
