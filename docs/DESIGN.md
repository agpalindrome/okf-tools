# deon design note: a colored deontic norm layer for accounting judgment

Status: exploratory design, representational-only. Rests on two paper spikes
([`spikes/revenue-recognition-timing.md`](spikes/revenue-recognition-timing.md),
[`spikes/lease-classification.md`](spikes/lease-classification.md)) and two seed
norm files ([`crates/deon/examples/`](../crates/deon/examples/)). No execution
semantics proposed yet
— see Non-goals.

deon is the judgment-side sibling to
**[Pacioli](https://github.com/ojhermann-org/pacioli)** (the verified Lean
mechanics); its bottom edge _is_ [Pacioli's seam][pacioli-seam], and "the Lean
seam" / "downstream Lean" throughout this note always refer to that library.

## 1. Purpose

A structured, checkable layer _inside_ the judgment side that sits between
**Open Knowledge Format** ([OKF][okf-spec]) prose and the plain-data seam Lean
consumes. It does not replace OKF and does not
touch Lean types. Its value is three things Lean-and-prose cannot give on their
own:

- **consistency** — detect conflicting obligations, coverage gaps, dead rules;
- **traceability** — from a proposed seam input back to the exact judgments it
  rests on;
- **auditable consequence-propagation** — "given these facts, this obligation
  follows, and it hinges on judgments P and Q, ungrounded."

It _assists_ Lean + OKF downstream; it is not a new authority. Lean proves the
mechanics are right given the inputs; deon shows the inputs are justified and
names the judgment chain.

## 2. Placement and hard constraints (bounded on both ends)

- **Top edge grounds in OKF prose + citations.** Open-textured predicates resolve
  there; the prose stays authoritative. If rule and prose disagree, prose wins.
- **Bottom edge grounds in the [Pacioli Lean seam][pacioli-seam].** Every
  well-formed norm terminates in a _commitment about plain data_ the kernel will
  check — the point where, per Pacioli's interface contract, a judgment produces
  plain-data _inputs_ and the Lean kernel consumes them deterministically. A norm
  that never constrains seam data is malformed.
- **Not in the pure core.** This is judgment-side / downstream tooling; it must
  not add an applied surface to the Lean library
  ([Pacioli #41][pacioli-41]) and must not put policy into a Lean type (the seam
  invariant), one level up.
- **The type system encodes the seam.** The mechanical/judgment cut is a checked
  property, as ["policy never leaks into types"][pacioli-seam] is on the Lean
  side — but by **declaration**, not derivation. Lean's version holds because
  the type is uninhabited without the input; deon's holds because the norm
  declared a color and the checker held it to that. The checked property is
  internal honesty — weaker than Lean's guarantee, and still worth having.

## 3. Abstract syntax (node types)

A **norm** is:

```
norm        := { id, regime, concept-ref, subject, deontic, defeat?, branches }
branches    := antecedent, covers?, commitment, otherwise?   # binary form
             | cases[]                                       # n-ary form
case        := { when?: antecedent, covers?, commitment }    # `when`-less = residual
regime      := standard/jurisdiction scope (e.g. ASC-840, IFRS-16)   # norms are regime-indexed
concept-ref := link to the governing OKF concept
subject     := the record-state the obligation ranges over (a PO, a lease, ...)
deontic     := obligation | permission | prohibition
covers      := state-id+        # which of the subject's declared states this branch handles
otherwise   := { covers?, commitment }   # residual: taken when the antecedent does NOT hold
```

`covers` is coverage's input (§4 check 3); the state ids it names are declared
for the subject in the OKF bundle, not here. A norm that names no state makes no
coverage claim and is not checked — and neither is a norm with no `subject:`,
since there is nothing to look a state space up by.

`regime` and `concept-ref` are **inherited** from the OKF document's frontmatter
(`regime:`, `concept:`) unless a norm sets its own — e.g. the lease document is
`ASC-840` but its `short-term-low-value-election` norm overrides to `IFRS-16`. A
norm without an explicit `otherwise` makes no commitment when its antecedent
fails (that silence is what the coverage check in §4 reasons about).

**Two branch forms, one meaning.** The **binary** form (`antecedent` +
`commitment` + optional `otherwise`) is sugar for the **n-ary** `cases:` form:
the antecedent is the first case's `when`, and `otherwise` is a trailing
`when`-less case. The n-ary form exists because a subject's states are not
always two: IFRS 15's performance obligation has a _third_ state — "not yet
satisfied → recognize nothing" — that an over-time/point-in-time binary cannot
express (spike 1, F5). Coverage (§4 check 3) reasons about exactly that gap, so
the language must be able to state the fix, not merely have the gap named.

Every check that reasons about a norm's commitments treats a case as a branch:
termination (§4 check 5) requires each case to carry its own commitment, and
conditional conflict (§4 check 4) collides against a middle case as readily as
against a residual.

The forms are **alternatives, not layers**: write a norm in one or the other.
Mixing them leaves the path to a commitment undefined, in three distinct ways,
and the checker flags all three (SEAM-3):

- a stray `antecedent` beside `cases:` is read by no check for branch structure,
  so it is dead text that still passes leak detection and grounding — while an
  author would reasonably read it as a **guard** ("the norm applies when this
  holds; within that, split into cases"). That is a plausible future meaning,
  deliberately _not_ defined here: a guard interacts with coverage (§4 check 3),
  which would have to decide whether cases must cover the subject's states or
  only the states inside the guard. Better to reject the shape now and define it
  when check 3 forces the question;
- an `otherwise` beside a `when`-less case gives the norm **two** mutually
  exclusive residuals, and which takes effect is unstated;
- a top-level `commitment` beside `cases:` reads as unconditional, contradicting
  the split.

**The invariant, stated once.** Anything that could fall on either side of the
seam must **say which**. A predicate, an `inputs:` entry, a `binds`, a
`threshold`, a commitment's `method`/`measure` — each carries an explicit
`color`, and silence is a defect rather than a default (LEAK-2/4, CONFLICT-4).
The checker cannot infer a color from a value: whether something is mechanical
is a standard-setter's policy choice, not a property of the thing (spike 2, N1).
`0.75 @ASC-840` and "major part" @IAS-17 are the same criterion, mechanized in
one regime and left open in the other; likewise IFRS 15.39's progress `measure`
is an open choice while the retrospective restatement IAS 8 requires for a
material error is determined. Only the norm knows, so only the norm can say.

This is also why `binds` above is a **colored predicate** and not a bare name:
priority is itself colored, so a form that cannot carry a color cannot express
what §3 requires of it.

**Predicates** (the antecedent is built from these) are _colored_:

```
predicate   := mechanical(test, inputs*)                 # decidable from seam data
             | judgment(grounds)                         # open-textured hole; must cite
             | election(grounds: entity-policy)          # a discretionary entity choice
             | violated(norm-id, status: mechanical|judgment)   # compliance as a predicate
```

- `mechanical` may take **inputs that are themselves judgment** (an estimate
  crossing the seam as a number). Its verdict is real but the trace runs through
  those inputs.
- `judgment(grounds)` — `grounds` has a **source type**: `standard-criterion |
  world-fact | legal-fact | entity-election`.
- `election(grounds)` is **not** a synonym for judgment, and the difference is
  checked: an election is a discretionary _entity_ choice, so its citation
  resolves to the entity's own policy — `source: entity-election`, enforced by
  GROUND-2. A choice the standard makes for you is a judgment, however it is
  labelled. Without this the three colors would be two wearing three names,
  since every other call site treats `election` and `judgment` alike.

  The converse is deliberately **not** checked, and is a live modelling
  question: the lease seed's `low-value` is colored `judgment` but sourced
  `entity-election`, because IFRS 16's exemption is an entity election whose
  _threshold_ ("low value") is open-textured. Whether that is one node or two —
  an election to apply the exemption, plus a judgment inside it — is a question
  about the norm, not the language, so the checker leaves it to the author.

**Connectives and aggregation:**

```
antecedent  := predicate
             | all-of[antecedent...] | any-of[antecedent...] | not(antecedent)   # mechanical glue
             | judgment-aggregation(factors*, grounds)   # weighed factors, NO combination rule
```

The `judgment-aggregation` node is load-bearing: it is how deon refuses to fake
a weighed judgment (e.g. IFRS 15.38 point-in-time indicators) as a formula.
Mechanical connectives may compose over judgment atoms — this is the core
discovery (a standard's _structure_ is mechanical, its _criteria_ judgment).

**Thresholds** are colored artifacts, not inert numbers:

```
threshold   := const(value) @regime        # e.g. 0.75 @ASC-840  (mechanized judgment)
             | judgment(grounds)            # e.g. "major part"   @IAS-17  (not mechanized)
```

**Commitment** (the bottom edge):

```
commitment  := { deontic-target, method?: judgment, estimate-inputs*: judgment,
                 modifies?: <field := capped/adjusted value> }
```

The commitment carries a _method choice + estimate inputs_ (both judgment); the
_arithmetic_ that turns them into per-period/per-account numbers is downstream
Lean's, not the norm's.

**Priority / defeat** is itself colored:

```
defeat      := { defeats: norm-id+, binds: colored-predicate, modifies?: <field := adjusted value> }
```

**Concrete rendering (how §3 maps to `crates/deon/examples/`).** §3 is
_abstract_ syntax; the
seed norms render it as OKF frontmatter YAML. Each lives in a `<concept>.okf.md`
file — OKF-format markdown: the norm block is the YAML frontmatter, the
authoritative cited prose is the body beneath it. The **`.okf.md` suffix is
load-bearing**: the checker collects norm files by it, while an OKF bundle's
concept files are plain `*.md`.

The document's frontmatter carries `concept`, `title`, `regime`, `sources`, and
**`norms:` — a sequence, and the only load-bearing key**. Anything else
(`coverage-note`, `regime-note`, …) is prose for the reader. Four of the six
checks walk that sequence directly, so a misspelled `norms:` means they silently
find nothing to check.

The field names differ from the abstract grammar in these deliberate ways:

- A commitment's `deontic-target` is written as the **domain field(s) it sets**
  — `classification: finance`, `timing: over-time`, `capitalize: false`,
  `adjustment: prior-period` — rather than the literal key `deontic-target`;
  the structural keys `method`, `estimate-inputs`, and `modifies` keep their
  grammar names where they appear. A `method`/`measure` is a **colored
  mapping**, never a bare value: write `{ value: retrospective, color:
  mechanical }` for a method the standard prescribes, and `{ color: judgment,
  grounds: … }` for an open choice. (An earlier draft of this note gave a bare
  `method: retrospective`, contradicting the `method?: judgment` production
  above — the drift LEAK-4 now catches.)
- Every colored node carries its color as an explicit **`color:` key** — the key
  every check reads, and the one the abstract productions above leave implicit.
  `judgment(grounds)` is written `{ predicate: <name>, color: judgment, grounds:
  { ref, source } }`; a mechanical predicate is `{ predicate: <name>, color:
  mechanical, test: <expr>, inputs?: { <name>: { color, source? } } }`, or the
  nested form `{ <name>: { mechanical: { test, inputs? } } }` the lease seed
  uses. `violated(norm-id, status)` renders its status **as the color**:
  `{ predicate: violated, norm: <id>, color: judgment, grounds: { … } }`. A
  `threshold` carries its regime stamp and its color as keys:
  `{ value: 0.75, regime: ASC-840, color: mechanical }`.
- A `defeat` appears in **either** of two shapes: as a **standalone** node
  (`{ defeats, binds, modifies? }`, e.g. `var-consideration-constraint`
  modifying the defeated norm's commitment), **or** as a **full norm carrying a
  `defeats:` field** with its own `antecedent`/`commitment` (e.g.
  `short-term-low-value-election`). Both are the `defeat` node above; the second
  just co-locates it with a norm that also stands on its own.

## 4. Static checks (representational; no execution)

1. **Leak detection.** No `judgment`/`election` predicate is evaluated as
   `mechanical`; every `mechanical` test's inputs are declared and colored.
   _Implemented_ as the `deon-check` crate (LEAK-1/2/3, plus LEAK-4: a
   commitment's `method`/`measure` must declare its color); run it with
   `nix run . -- crates/deon/examples/`.

   These rules verify that an author was internally **consistent** about the
   seam — a name declared judgment is not then computed on, and computed-on data
   was declared. They cannot verify that a `mechanical` predicate _deserves_ the
   color: `lease.is-specialized` is indistinguishable from any other boolean
   field of the subject without the accounting knowledge this checker
   deliberately does not hold.
2. **Grounding completeness.** Every `judgment` hole has a `grounds` ref that
   resolves to a real OKF concept anchor, with a declared source type.
   _Implemented_ in `deon-check` (GROUND-1/2 always; GROUND-3 anchor resolution
   under `--okf <bundle>`). A hole is a **criterion** — a judgment/election
   predicate, threshold, `judgment-aggregation`, `binds`, or commitment
   `method`/`measure` — which resolves to concept prose and so needs a `ref`. A
   judgment-colored **input** to a mechanical test is a _value_ (an estimate): it
   resolves to runtime evidence, not norm-time prose, so it is grounded by its
   `source` type alone and needs no `ref` (issue #10).
3. **Coverage.** The antecedent branches partition the subject's relevant states;
   flag implicit gaps (spike 1 found "PO not yet satisfied → recognize nothing"
   was unrepresented). An `otherwise` branch makes the split _syntactically_
   total (antecedent-holds → `commitment`, else → `otherwise.commitment`), but
   coverage still checks that this binary matches the subject's real states — the
   "recognize nothing" third state is a gap an over-time/point-in-time `otherwise`
   hides, not one it closes. _Implemented_ in `deon-check`: COVER-1 a declared
   state no branch of the norm claims; COVER-2 a `covers:` naming a state the
   subject does not declare; COVER-3 a bundle state declaration that names no
   state; COVER-4 a bundle block that reads as a state space but yields none.

   The checker cannot know a subject's states — deriving them would be an
   evaluator (§9) or a hardcoded accounting fact (out of scope). So the **state
   space is declared**, as norm content in the concept bundle beside the prose
   that grounds it, and coverage runs only under `--okf` (like GROUND-3):

   ```yaml
   subjects:
     performance-obligation:
       states:
         - { id: not-yet-satisfied, grounds: { ref: "#ifrs15-31", source: standard-criterion } }
   ```

   The declaration is itself a judgment about the standard, so it is held to
   deon's rule for a judgment: it must cite. The bundle's own state declarations
   are checked once per bundle — COVER-3 for a declaration naming no state,
   COVER-4 for a block that reads as a state space but yields none (unparseable
   frontmatter; a subject with no `states:` list), and GROUND-1/2/3 for a
   declaration's citation. COVER-3 and COVER-4 are the same failure at two
   altitudes: something that reads as a state space silently yields nothing, and
   a state absent from the space is a state coverage stops looking for. A
   concept file with _no_ frontmatter is not a defect — a prose-only concept
   file is ordinary.

   A branch claims a state with `covers:` — on the norm (the antecedent-holds
   branch), on `otherwise`, or on each `cases[i]`. **Coverage is opt-in per
   norm:** a norm that claims no state is skipped, since nothing can be said
   about whether its branches are total; once a norm claims one state it must
   claim them all. A norm with no `subject:` is skipped for the same reason —
   without a subject there is no state space to check against. The rev-rec seed
   carries `covers:` tags and its third state is left unrepresented on purpose,
   so COVER-1 reports it — the finding spike 1 predicted (F5), kept live rather
   than papered over.
4. **Conditional conflict.** When norm A `defeats` norm B via a judgment `binds`,
   report the conflict as _underdetermined until grounded_, not a static
   contradiction. _Implemented_ in `deon-check`: CONFLICT-1 a `defeats:` naming
   no norm in the document; CONFLICT-2 a **collision** — defeater and defeated
   constraining the same commitment field with different values — whose `binds`
   is judgment/election-colored, reported as `underdetermined(<predicate>)`;
   CONFLICT-3 the same collision bound mechanically, so decidable at the seam and
   reported as a real conflict; CONFLICT-4 the same collision where `binds`
   carries no color at all — priority is itself colored, so an uncolored
   predicate cannot be called decidable (the bare `binds: <predicate>` form
   above cannot carry a color, so it always lands here). A defeat edge that
   collides on nothing, or that carries no `binds` at all (unconditional
   priority), is not a conflict.

   `defeats:` and `covers:` take a **list**; a bare scalar is sugar for a
   one-element list, exactly as `otherwise` is sugar for a `when`-less case.
   They are recognized in **any** shape a norm writes them.
   The list forms are accepted and meaningful — one norm may defeat several, and
   one branch may cover several states — and a shape that names nothing is
   reported. Neither may be silently skipped: a claim the checker cannot read
   must never read as "this norm makes no claim", or a typo becomes an opt-out
   of the check that would have caught it.
5. **Termination-at-seam.** Every norm's obligation reaches a `commitment` about
   plain data; flag any that don't. _Implemented_ in `deon-check` (SEAM-1 a norm
   with neither a `commitment` nor a `modifies`; SEAM-2 an empty commitment or an
   `otherwise` residual branch that carries none; SEAM-3 a norm mixing the
   binary and n-ary branch forms of §3, whose path to a commitment is then
   undefined).
6. **Regime hygiene.** A norm applies only within its `regime`; flag facts
   evaluated against a norm whose regime doesn't apply (e.g. lessee
   classification under IFRS-16 — the norm doesn't exist there). The
   facts-against-wrong-regime facet needs runtime facts (deferred, §9); the
   representational facet is _implemented_ in `deon-check`: REGIME-1 a norm with
   no effective regime (own or inherited), REGIME-2 a `@regime`-stamped artifact
   (a threshold) that doesn't match its norm's regime. Cross-regime `defeats`
   edges are not flagged — the lease seed uses one deliberately to model
   regime-relativity.

**The bundle contract.** Two checks (GROUND-3 and coverage) read norm content
that lives beside its cited prose rather than in a norm file: the **anchors** a
citation can resolve to, and each subject's **declared state space**. That
requirement is deon's own, and is stated as such — an OKF bundle is the intended
carrier and the only binding implemented (`src/okf.rs`), but the contract does
not wait on an upstream format to settle. The alternative was to leave a third
of the checker blocked on a spec in another project's repo.

## 5. Signature capability (the reason to build it)

Run the commitment backward: **given a proposed set of seam inputs, emit the
trace of judgments they depend on, and flag any unresolved or conflicting ones.**
Verdicts are three-valued: `complied | violated | underdetermined(P...)`. Even a
purely mechanical bright-line test traces _through_ its estimate inputs to the
judgments beneath (spike 2, N2).

## 6. Where "neuro" attaches (kept out of the symbolic core)

- **Extraction** (prose → candidate norms): assistive, human-ratified. Low risk.
- **Grounding** the open-textured holes at evaluation time ("is control
  transferred?", "is it material?"): higher stakes — must surface _which_ OKF
  concept + citation it grounded against, never decide invisibly.
- **Conflict-resolution suggestions**: detection is symbolic; only suggestions
  are neural.

The symbolic layer stays auditable; neural contributions are always explicit,
cited, and reviewable — never baked into a rule.

## 7. Evidence (two converging spikes)

- **Rev-rec timing** (open-textured): established the core — mechanical
  connectives over judgment atoms; judgment-aggregation node; contrary-to-duty
  for free; colored-judgment priority; method+estimate commitments; a real
  coverage gap found statically.
- **Lease classification** (bright-line, adversarial contrast): four of five
  features held verbatim; growth was one new feature — regime-scoped norms +
  colored threshold constants — which is exactly the axis a bright-line concept
  probes. The space is bracketing, not exploding.

**Spec-able core:** (1) colored predicates — mechanical / judgment / election;
(2) grounding-source taxonomy; (3) mechanical connectives over judgment atoms;
(4) judgment-aggregation nodes; (5) regime-scoped norms + colored thresholds;
(6) three-valued compliance; (7) colored-judgment priority; (8) commitments =
method + estimate bundles feeding downstream Lean.

## 8. Open questions

- **Regime as a first-class dimension.** Norms differ by ASC 840 / IAS 17 / IFRS
  16, sometimes existing under one and not another. How much regime machinery is
  warranted before it becomes its own project?
- **Grounding-source taxonomy** — is `{standard-criterion, world-fact,
  legal-fact, entity-election}` complete, and does each source want different
  handling (legal facts may reference law, not accounting literature)?
- **Priority when the ordering itself conflicts** — colored-judgment priority
  handles "does A defeat B," but not yet "two defeats disagree."
- **Overlap with existing standards** (SBVR modalities, LegalRuleML, Governatori's
  defeasible deontic logic) vs. what is genuinely ours (the two bounded edges).
  Adopt where we can; the seam-anchoring is the novel part.
- **The seam contract is asserted, not checked.** SEAM-1/2 verify a norm reaches
  a non-empty commitment, not that what it commits to is a value the kernel can
  consume — `commitment: { classification: vibes }` passes today. A subject's
  admissible commitment values are norm content of the same kind as its state
  space, so the bundle contract is the natural place to declare them
  ([probe](probes/ias32-liability-vs-equity.md), finding 2).
- **A measurement layer sits between the repos, owned by neither.** deon commits
  to a _policy selection_ (`classification: liability`, `timing: over-time`);
  Pacioli's kernel consumes numbers plus a classification, treating a schedule as
  plain-data input rather than deriving it. IAS 32's compound instruments make
  this concrete: one instrument becomes two classified accounts, split by an
  allocation neither repo computes (probe, finding 3).
- **"States" may be the wrong word for what coverage partitions.** It fits a
  subject that moves through states (a performance obligation) and strains on one
  that merely has terms (a financial instrument) (probe, finding 4).

## 9. Non-goals (deliberately deferred)

- **No execution semantics yet.** Static analysis over a normal form is the first
  deliverable; a facts→verdict engine is a much bigger commitment, earned later.
- **No concrete/custom syntax yet.** Begin as a disciplined schema inside OKF
  frontmatter so every norm sits beside its cited prose.
- **No neural components built.** Section 6 marks where they attach; none are
  proposed for the first cut.
- **Nothing lands in the pure Lean core.**

[okf-spec]: https://github.com/GoogleCloudPlatform/open-knowledge-format/blob/main/SPEC.md
[pacioli-seam]: https://github.com/ojhermann-org/pacioli#the-interface-contract-the-crux
[pacioli-41]: https://github.com/ojhermann-org/pacioli/issues/41
