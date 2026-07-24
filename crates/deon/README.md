# deon — a colored deontic norm language for accounting judgment

From Greek _déon_, "that which is binding" — the root of _deontic_. A small
language and static checker for **accounting judgments and their consequences**,
sitting between curated judgment prose (the **Open Knowledge Format**,
[OKF][okf-spec]) and a verified mechanical core (the
[Pacioli](https://github.com/ojhermann-org/pacioli) Lean library). It captures rules
of the shape "you must do X when Y" — obligations, permissions, prohibitions — in
a form that is **consistency-checkable, traceable, and auditable**, while every
actual judgment stays an explicit, cited hole rather than a hardcoded policy.

## What it is (and is not)

deon is the **language and its checker** — the semantics and tooling. It is _not_
a collection of norms and it is _not_ an authority on accounting. Norm _content_
lives beside its cited prose in an OKF judgment bundle; deon _consumes_ those
bundles and checks them. It ships a small seed set of worked examples
([`examples/`](examples/) — revenue-recognition timing, lease classification)
until an OKF bundle exists to hold them.

This mirrors [the split][pacioli-split] its sibling **Pacioli** keeps on the
mechanical side: Pacioli owns the verified _machinery_, not the accounting data;
deon owns the _norm language_, not the norms.

## Why it exists

Lean proves the mechanics are right _given_ the inputs. OKF prose says _what_ the
inputs should be and _why_. Neither shows that a proposed set of inputs is
**justified** — nor names the chain of judgments it rests on. That gap is what
deon fills:

- **consistency** — conflicting obligations, coverage gaps, dead rules, found
  statically;
- **traceability** — from a proposed seam input back to the exact judgments
  beneath it;
- **auditable consequence-propagation** — "this obligation follows, and it hinges
  on judgments P and Q, currently ungrounded."

deon _assists_ Lean + OKF; it adds no new authority over either.

## The core idea: the seam is in the type system

Every predicate is **colored** — `mechanical` (decidable from seam data),
`judgment` (open-textured, must cite where it grounds), or `election` (a
discretionary entity choice). A standard's logical _structure_ is mechanical; its
_criteria_ are judgment — and deon lets mechanical connectives compose over
judgment atoms so both live in one rule. The invariant the checker holds you to
is that **anything which could fall on either side of the seam must say which**:
a colored name is never silently computed on, every judgment hole carries a
citation, and silence about a color is a defect rather than a default.

**What a green check does and does not buy.** It says the formalization is
_internally honest_ — the norm is consistent with itself about where judgment
lives. It does **not** say the accounting is right; prose always wins, and a
norm that disagrees with it is the bug. The guarantee is narrower than Lean's on
[the other side of the seam][pacioli-seam]: Lean's policy-never-leaks holds
because the type is uninhabited without the input, while deon's holds because
you declared the color and the checker kept you to it. That is the difference
between derivation and declaration, and it is worth being plain about — a
judgment can still be laundered by writing it as a field of its own subject, and
no static check can catch that without the accounting knowledge deon
deliberately does not hold.

Full abstract syntax and static checks: **[docs/DESIGN.md](../../docs/DESIGN.md)**.
The two worked spikes that this design rests on:
**[docs/spikes/](../../docs/spikes/)**.

## Bounded on both ends

- **Top** grounds in OKF prose + citations; prose stays authoritative.
- **Bottom** grounds in the [Pacioli Lean seam][pacioli-seam]: every well-formed
  norm terminates in a commitment about plain data the kernel checks. A norm that
  never constrains seam data is malformed.

## Scope discipline (borrowed from Pacioli, deliberately)

- **Representational first.** Static analysis over a normal form is the first
  deliverable. A facts→verdict execution engine is a later, earned commitment.
- **No custom syntax yet.** Norms begin as a disciplined schema inside OKF
  frontmatter, beside their prose. The bespoke DSL syntax is a later ergonomic
  layer over a validated semantics.
- **Neural stays at the edges.** Extraction (prose → candidate norms) and
  grounding (resolving open-textured holes at evaluation time) are assistive and
  always cited/reviewable — never baked into a rule. The symbolic core stays
  auditable.

## The checker, so far

**`deon-check`** walks the OKF-frontmatter norm schema and runs all six DESIGN
§4 checks — both edges of the seam, plus coverage and priority:

- **Leak detection** (check 1) — the mechanical edge: a `mechanical` test
  computing on a judgment (LEAK-1), an undeclared/uncolored input (LEAK-2), a
  `judgment-aggregation` faked as a formula (LEAK-3), or a commitment
  `method`/`measure` that declares no color (LEAK-4).
- **Grounding completeness** (check 2) — the judgment edge: every judgment
  criterion is cited (GROUND-1) and its source typed (GROUND-2); with `--okf`,
  its `ref` resolves to a real anchor (GROUND-3).
- **Coverage** (check 3, `--okf`) — the branches partition the subject's
  declared states: a state no branch claims (COVER-1), a `covers:` naming a
  state the subject does not declare (COVER-2). The bundle is held to the same
  standard — a state declaration naming nothing (COVER-3), or a block that reads
  as a state space but yields none (COVER-4).
- **Conditional conflict** (check 4) — priority is itself colored: a `defeats:`
  pointing nowhere (CONFLICT-1), a collision left _underdetermined until
  grounded_ by a judgment `binds` (CONFLICT-2), one decidable at the seam
  (CONFLICT-3), and one whose `binds` declares no color, so it is neither
  (CONFLICT-4).
- **Termination-at-seam** (check 5) — the bottom edge: every norm reaches a
  commitment about plain data (SEAM-1/2), by a well-defined path (SEAM-3).
- **Regime hygiene** (check 6) — a norm's mechanized artifacts belong to its
  regime (REGIME-1/2).

```sh
nix run . -- crates/deon/examples/                    # the seed norms → no defects (exit 0)
nix run . -- crates/deon/tests/fixtures/leaky.okf.md  # red: 3 located leaks (exit 1)

# The bundle-backed checks. This bundle declares the performance-obligation
# state space, so COVER-1 reports the seed's deliberate gap — the "PO not yet
# satisfied → recognize nothing" third state spike 1 predicted (F5).
nix run . -- --okf crates/deon/tests/fixtures/okf-states crates/deon/examples/
```

Every check ships with both a green case (the seed norms, authored honestly) and
a red fixture, because a checker you've only seen say "clean" isn't a checker.
`nix flake check` builds, lints, and tests it.

## Status

Exploratory. The design rests on two converging paper spikes
([`docs/spikes/`](../../docs/spikes/)) and the design note
([`docs/DESIGN.md`](../../docs/DESIGN.md));
[`examples/`](examples/) holds the seed norm files. All
six DESIGN §4 checks are built (coverage and GROUND-3 need an OKF bundle, via
`--okf`); no execution engine or neural components are built yet — see the
design note's Non-goals.

[okf-spec]: https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md
[pacioli-split]: https://github.com/ojhermann-org/pacioli#why-this-split
[pacioli-seam]: https://github.com/ojhermann-org/pacioli#the-interface-contract-the-crux
