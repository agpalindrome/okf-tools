# okf-graph — topology and identity for an OKF bundle

A structural / topological validator for an [Open Knowledge Format][okf-spec]
(OKF) Knowledge Bundle. Given a bundle, `okf-graph` reads it as a knowledge
graph and reports — quickly and rigorously — what is structurally wrong with
it: what Concepts exist, what they are called, what points at what, what
resolves and what dangles. A concept's body is carried as an _opaque payload_
and never interpreted.

## What it is (and is not)

`okf-graph` is a checker over _structure_, not an authority on _content_. Two
limits follow, and both are load-bearing:

- **Structure is not fidelity.** That a bundle is well-formed says nothing about
  whether it faithfully captures the knowledge it claims to. A bundle can be
  fully conformant and wrong; expert review is that gate, and `okf-graph` does
  not stand in for it.
- **Topology is not meaning.** `okf-graph` reads _that_ one concept points at
  another, never _what_ the relationship means. OKF links are untyped, and the
  meaning of a norm — obligation, defeat, the mechanical/judgment seam — is
  `okf-normative`'s to read, one layer up. `okf-normative` depends on
  `okf-graph`, never the reverse.

## What it checks

Every rule the spec makes checkable ships as a _located finding_, at two levels:

- **Per-concept** reads a single document against [§4]'s shape and the optional
  [§5] families — a present, non-empty `type`; frontmatter field shapes; the
  provenance / trust / lifecycle metadata; the actor convention ([§7]); and the
  Attested-Computation contract ([§10]).
- **Whole-bundle** reads the graph — unique Concept IDs and reserved-file
  exclusion, body-link resolution, the path-valued fields and the `references/`
  convention, the provenance / derivation graph and its cycles, and the
  structure of the reserved `index.md` and `log.md`.

A finding is either a **defect** to fix or a **report** about something the spec
says to tolerate — a dangling link, a missing optional family. OKF's consumption
model is permissive ([§11]), so a report is surfaced and printed but does not
fail the run; only defects do. The full model — the edge kinds and why acyclicity
is checked per kind, the defect/report cut, the deferred boundary — is in the
[design note](../../docs/okf-graph-DESIGN.md).

## Run it

`nix run .#okf-graph -- <bundle>` validates one bundle directory (searched
recursively for concept files). `nix run .` stays `deon-check`; this binary is
its sibling. Exit codes: `0` = no defects (reports may still print), `1` = one
or more defects, `2` = usage / IO error.

```sh
# A clean bundle → no findings (exit 0).
nix run .#okf-graph -- crates/okf-graph/tests/fixtures/clean

# A defect → the run fails (exit 1).
nix run .#okf-graph -- crates/okf-graph/tests/fixtures/missing-type
# untyped.md  CONCEPT-2 (missing type): concept declares no non-empty `type`

# A tolerated report → printed, but the run still passes (exit 0).
nix run .#okf-graph -- crates/okf-graph/tests/fixtures/dangling
# note.md  BUNDLE-2 (dangling link): link resolves to no concept in the bundle
```

Every check ships with both a green case and a red fixture, because a checker
you have only seen say "clean" is not a checker. `nix flake check` builds,
lints, and tests the workspace.

## Status

Exploratory, structural-only, and built out. The design note
([docs/okf-graph-DESIGN.md](../../docs/okf-graph-DESIGN.md)) is authoritative for
topology and identity. Where the OKF format fights the tools, the concrete cases
are logged — with dates — in the [friction log](../../docs/okf-friction.md), raw
material for an eventual upstream conversation.

[okf-spec]: https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md
[§4]: https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md#4-concept-documents
[§5]: https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md#5-provenance-trust-and-lifecycle
[§7]: https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md#7-actor-convention
[§10]: https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md#10-attested-computations-concept
[§11]: https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md#11-conformance
