# Spike 2: lease classification (the bright-line contrast)

Concept: classify a lease as **finance** or **operating** (lessee). Chosen as the
deliberate opposite of rev-rec timing: the standard's showcase for _bright-line_
mechanical tests, to see whether the spike-1 feature list holds or grows.

## Standards are not one thing here (this is the whole point)

- **ASC 840** (superseded US GAAP): explicit bright lines — term ≥ **75%** of
  economic life; PV of payments ≥ **90%** of fair value.
- **ASC 842** (current US GAAP): keeps finance/operating for lessees; reworded
  criteria, removed the explicit 75%/90% but permits them as "one reasonable
  approach."
- **IAS 17** (superseded IFRS): "substantially all risks and rewards"; 75%/90%
  are illustrative indicators, not strict tests.
- **IFRS 16** (current IFRS): lessees capitalize **all** leases — the
  finance/operating classification norm **does not exist** for lessees (lessors
  still classify).

## The five classic criteria — finance lease if ANY hold

| Criterion | Color | Note |
| --- | --- | --- |
| ownership transfers by end of term | mechanical (contract fact) | decidable from an input flag |
| purchase option reasonably certain | judgment | probabilistic lessee-behaviour assessment |
| term ≥ major part of economic life | **regime-dependent** | ASC 840: `≥ 0.75` mechanical; IAS 17: "major part" judgment |
| PV of payments ≥ substantially all of fair value | **regime-dependent** | ASC 840: `≥ 0.90` mechanical; IAS 17: judgment |
| specialized asset, no alternative use | judgment | same concept as IFRS 15.35(c) |
| `is-finance-lease` = ANY of the above | **mechanical** | five-way disjunction — same shape as `over-time` |

Inputs to the "mechanical" tests are estimates: economic life, fair value, and
the discount rate are all judgment.

## Findings vs. the spike-1 feature list

**Held (stable across both concepts):** mechanical connective over atoms (the
five-way disjunction); commit = classification, Lean does measurement;
contrary-to-duty via reassessment + violation-as-judgment; colored-judgment
priority (the exemption election).

**NEW — N1 (a genuinely new structural feature): threshold-as-colored-artifact,
and coloring is standard-relative.** The _same_ criterion is mechanical `≥ 0.75`
under ASC 840 and pure judgment ("major part") under IAS 17. Whether a criterion
is a bright line or a judgment is a _standard-setter's policy choice to mechanize
a judgment_. The language must color the threshold constant itself and scope the
whole norm by regime. Strongest demonstration: under IFRS 16 the lessee
classification norm **does not exist at all** — a norm's existence is
regime-relative.

**Refinements (not new features):**

- N2 — a "mechanical" bright-line test can have every input be a judgment
  estimate; the dependency trace must reach _through_ it to those inputs.
  Reinforces the trace capability.
- N3 — grounding-source taxonomy: judgment holes ground in different _kinds_ of
  source — standard criteria, world-facts, legal facts, entity elections. The
  `grounds:` field has a type, not just a URL.
- N4 — deontic operator set is at least {obligation, permission}; the election
  exercised permission, which reused all node types.

## Verdict: the space is bracketing, not exploding

Across an open-textured concept (rev-rec) and a bright-line concept (leases) the
core features held. Growth was one new structural feature (N1) plus two
refinements — and N1 is exactly the axis a bright-line concept was chosen to
probe. The list is converging.
