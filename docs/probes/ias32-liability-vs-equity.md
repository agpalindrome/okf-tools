# Probe: IAS 32 liability-vs-equity, and the Pacioli seam

A **probe**, not a spike. The two `docs/spikes/` notes were written *before* the
language, to derive it. This was written *after* encoding
[`examples/financial-instrument-classification.okf.md`](../../examples/financial-instrument-classification.okf.md),
deliberately with no design note first — the point was to test the language
against a standard it was not designed from, and to record what happened rather
than what we hoped would happen.

Two hypotheses, one piece of work:

1. **Does the language survive a third concept?** Both spikes came off the same
   shelf — revenue and leases, the two most-formalized standards in existence,
   both with clean subject-level state machines. Everything built so far has
   only been tested on cases it was designed from.
2. **Is deon's bottom edge Pacioli's seam?** The repos assert it; nothing
   exercises it. IAS 32 is the cheapest possible test, because its commitment is
   a classification verdict — precisely the argument Pacioli's injected
   `classify : α → AccountClass` wants (`Pacioli/Classification.lean`, whose own
   docstring says the assignment is "supplied from the OKF half").

## What went right

**The checker said something true that its author had not already worked out.**
Encoding produced two `CONFLICT-2` findings without being designed to:

```
norms[1].defeats  CONFLICT-2: underdetermined(contingency-is-genuine) —
  `contingent-settlement` defeats `instrument-classification` on
  `classification` (`liability` vs `equity`)
norms[2].defeats  CONFLICT-2: underdetermined(meets-puttable-conditions) —
  `puttable-exception` defeats `instrument-classification` on
  `classification` (`equity` vs `liability`)
```

Both are correct and are the interesting part of IAS 32: whether a contingent
settlement provision makes an instrument a liability (32.25), and whether the
puttable exception rescues it into equity (32.16A–16D), cannot be settled
statically — each turns on a judgment. The three-valued report is the right
answer, and it fell out of the representation rather than being put there.

The coloring also held. Nothing had to be frozen: `fixed-for-fixed` crossed the
seam as a judgment **input** to a mechanical test (a legal reading of contract
terms, consumed as a value), which is exactly the shape spike 2 (N2) predicted.

## Finding 1 — an underdetermined conflict was failing the run

`nix run . -- examples/` exited **1** once this seed landed, because
`CONFLICT-2` counted as a finding like any other. But DESIGN §4.4 says in as many
words that an underdetermined conflict is reported *"not as a static
contradiction"* — and here it arises from modelling IAS 32 **correctly**. A
correctly modelled defeasible norm could never have passed the checker.

The three-valued report was collapsing back to pass/fail at the exit code.
Fixed: findings now carry a `Severity`, `CONFLICT-2` is a `Report`, and only
`Defect`s fail the run. Reports still print.

This is the probe's most useful result, and it was invisible until a norm whose
*correct* encoding produces an underdetermined conflict actually existed.

## Finding 2 — the seam contract is asserted, never checked

SEAM-1/2 verify that a norm reaches a **non-empty commitment**. They do not
verify that what it commits to is something the kernel could consume. Replacing
the classification with a nonsense value:

```yaml
commitment: { classification: vibes }
```

produces **zero** findings. So the claim that "every well-formed norm terminates
in a plain-data commitment the Pacioli kernel checks" is, today, unverified in
both repos: deon checks presence, Pacioli assumes a total `classify`, and
nothing connects the two.

The bundle contract is the obvious place to fix it — a subject's admissible
commitment values are norm content of exactly the same kind as its state space,
and would be declared beside the prose. Not built here; recorded as an open
question (§8).

## Finding 3 — compound instruments have no representation, and the gap is real

IAS 32.28–32 splits a convertible bond into a liability component **and** an
equity component, allocating the carrying amount by the residual method. deon
cannot say that. A norm commits to *one* classification, so the seed writes:

```yaml
commitment:
  classification: compound
  method: { value: residual-allocation, color: mechanical }
```

which is a placeholder, not a modelling. `compound` is not a value
`AccountClass` has — the instrument becomes **two accounts**, and the split
between them is an allocation computed from a fair-value measurement.

That is the measurement layer, and it is owned by neither repo. deon's commitment
is a *policy selection*; Pacioli's kernel consumes *numbers and a classification*
and treats a schedule as plain-data input rather than deriving it. Between
"classify this as compound, allocate by residual method" and "here are two
classified accounts with amounts" sits arithmetic that spike 1 (F4) assigned to
"downstream Lean" while Pacioli's interface contract assigns it upstream.

So the answer to hypothesis 2 is **qualified yes**: the first three norms
terminate exactly on the seam, and the fourth shows the seam is narrower than
"every norm's commitment". Some commitments are seam-terminal; others are one
layer up and need a measurement step that exists nowhere.

## Finding 4 — coverage's vocabulary strains on a static subject

Coverage asks whether the branches partition "the subject's relevant states".
For a performance obligation that reads naturally: a PO moves through states, and
the missing one (not-yet-satisfied) was a real gap. A financial instrument does
not move through states — it *has terms*, and the norm partitions those. The
check still applies (the outcomes are mutually exclusive and exhaustive), but
"state" is the wrong word for what is being partitioned, and the third-state gap
that motivated the check has no obvious analogue here.

No change made. Recorded because the concept may need renaming if a fourth
concept strains it the same way.

## What was not tested

This encoding is a **test artifact, not an authority on IAS 32**, and a reviewer
who knows the standard better will find renderings to argue with — particularly
whether `contractual-obligation-to-deliver-cash` deserves to be a single
judgment atom, and whether the puttable conditions (32.16A) should be decomposed
rather than hidden behind one `binds` predicate. Neither affects the findings
above.
