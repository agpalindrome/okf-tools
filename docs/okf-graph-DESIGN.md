# okf-graph design note: topology and identity for an OKF bundle

Status: exploratory, structural-only. `okf-graph` reads an
[Open Knowledge Format][okf-spec] (OKF) Knowledge Bundle as a knowledge graph
and reports what is structurally wrong with it. This note is the crate's design
record — what it models, what it checks, and the boundaries it holds — and is
edited as the crate is built out. It is authoritative for topology and identity;
where it and the working [okf-stack board][board] disagree on those, this note
wins.

## 1. Purpose

`okf-graph` answers the structural questions about a bundle: what Concepts
exist, what they are called, what points at what, what resolves and what
dangles. Given a bundle it identifies — quickly and rigorously — the structural
and topological problems in it, at two levels:

- _per-concept_ (document-level): a single concept that is malformed, missing a
  required field, or internally inconsistent about its own metadata;
- _whole-bundle_ (topology): a duplicated identity, a link that resolves to
  nothing, a reserved file in the wrong shape, a derivation that cycles.

It is a checker over structure, not an authority on content. Two limits follow,
and both are load-bearing:

- **Structure is not fidelity.** That a bundle is well-formed says nothing about
  whether it faithfully captures the knowledge it claims to. A bundle can be
  fully conformant and wrong; expert review is that gate, and `okf-graph` does
  not stand in for it.
- **Topology is not meaning.** `okf-graph` reads _that_ one concept points at
  another, never _what_ the relationship means. OKF links are untyped ([§6]),
  and the meaning of a norm — obligation, defeat, the mechanical/judgment seam —
  is `okf-normative`'s to read, one layer up.

## 2. Placement and the boundary

The order of precedence across the stack is **okf > okf-graph > okf-normative >
applications**. OKF the spec is upstream of everything: `okf-graph` matches its
structure and adapts as it changes, rather than bending the spec to what is
already built. Inside the repo, topology is upstream of semantics — `okf-graph`
carries a concept's body as an _opaque payload_ and never interprets it, and
`okf-normative` depends on `okf-graph`, never the reverse.

The boundary is easy to violate inside a single workspace, so it is stated as a
falsifiable test, inherited from the crate's `lib.rs`: **if this crate's test
suite cannot be read by someone who has never heard of deontic logic, the
boundary is not real.** No color, no norm, no seam appears in an `okf-graph`
fixture or test name. The archived `deon` crate is _reference, not oracle_: it
implements an earlier, normative reading, and where it and this note disagree,
this note wins and `deon` stays as it is.

## 3. The model

A **Concept** is one markdown document, read as a `Frontmatter` block and a
`Body` ([§4]). The body is an _opaque payload_ this crate never interprets.

Identity is **bundle-owned**. A Concept ID is the concept file's path within the
bundle with the `.md` suffix removed ([§2]) — the file `tables/orders.md` has
Concept ID `tables/orders`.

A **Bundle** is the set of concepts a directory tree yields, plus the reserved
files that are _not_ concepts — `index.md` and `log.md`, which carry defined
meaning at any level ([§3.1]) and are validated as structures of their own
rather than read as concepts.

## 4. Edges: kind by origin

OKF links carry no type: the kind of relationship "is conveyed by the
surrounding prose, not by the link itself" ([§6]).

But a bundle _does_ carry structurally distinct kinds of reference, and the
distinction comes from **where a reference sits, not from any link syntax**. An
edge takes its kind from its origin field:

- **body-link** — a markdown link in a concept body ([§6.1]): the untyped
  relationship edge, the one the spec describes.
- **parent/child** — implicit in the directory tree ([§3]): not a link at all,
  but a real structural edge between a concept and its enclosing scope.
- **resource** — a concept's `resource` ([§4.1]): the external or internal asset
  the concept is _about_.
- **sources[].resource** — provenance ([§5.1]): where a concept's content
  derives from; a derivation edge when it points at another concept.
- **computation / executor.resource / attester.resource** — the path edges of an
  Attested Computation ([§10]).

Properties are checked **per edge kind, not over the graph as a whole** because
the algebra is different for each: parent/child is a tree, acyclic by
construction; derivation ([§5.1]) must not cycle, or credibility propagation
would not terminate; body-links may cycle harmlessly, and are even allowed to
dangle ([§6]); `resource` and the [§10] path edges are many-to-one. A single
global acyclicity rule would reject a correct bundle.

## 5. Severity: defect vs tolerated report

A finding is either a **defect** to fix or a **report** about something the spec
says to tolerate.

The distinction is the spec's, not an ergonomic nicety. OKF's consumption model
is permissive ([§11]) and names cases a consumer **MUST NOT** reject over:

- a **dangling link** — [§6] requires tolerating a broken link, which "may
  simply represent not-yet-written knowledge";
- a **missing optional family** — [§5.3] / [§11] forbid rejecting a concept for
  lacking provenance, trust, or lifecycle metadata.

These are _reports_: surfaced and printed so nothing is silently dropped.
Malformed structure and missing _required_ fields — no `type` ([§11]), a
`sources` entry with no `resource` ([§5.1]), `generated` with no `by`
([§5.2]) — are _defects_.

## 6. What is checked

Every rule the spec makes checkable ships as a located finding — a defect or a
tolerated report. This section is the shape, not the registry.

**Per-concept** reads a single document against [§4]'s shape and the optional
families: `type` present and non-empty ([§11]); the field shapes of [§4.1]; the
provenance, trust, and lifecycle families ([§5]) — a `sources` entry's required
`resource`, the `verified` singleton that counts as one and not zero ([§5.2]),
the actor convention ([§7]), the `at` timestamps read as RFC 3339 rather than as
ISO 8601 entire (see the [friction log](okf-friction.md)), and `status` /
`stale_after` ([§5.4], [§5.5]); and
the Attested-Computation contract ([§10]) — `runtime`, typed `parameters`, and
the computation-or-fence exclusivity ([§10.3]).

**Whole-bundle** reads the graph: unique Concept IDs and reserved-file exclusion
([§3.1]); body-link resolution with dangling links tolerated ([§6]); the
path-valued fields ([§6.2]) and the `references/` convention ([§6.3]); the
provenance and derivation graph and its cycles ([§5.1]); and the structure of
the reserved `index.md` and `log.md` ([§8], [§9]), including a declared
`okf_version` ([§12]).

## 7. Deferred

Out of scope here, by the layer boundary or by the nature of the problem:

- **Meaning** — is downstream. No evaluation engine, bespoke syntax, or neural
  component belongs in this crate.
- **Fidelity** — whether a bundle faithfully encodes the standard it claims is
  not machine-checkable; expert review is that gate.

[okf-spec]: https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md
[board]: https://github.com/orgs/ojhermann-org/projects/8
[§2]: https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md#2-terminology
[§3]: https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md#3-bundle-structure
[§3.1]: https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md#31-reserved-filenames
[§4]: https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md#4-concept-documents
[§4.1]: https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md#41-frontmatter
[§5]: https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md#5-provenance-trust-and-lifecycle
[§5.1]: https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md#51-provenance-sources
[§5.2]: https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md#52-trust-generated-and-verified
[§5.3]: https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md#53-trust-tiers
[§5.4]: https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md#54-lifecycle-status
[§5.5]: https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md#55-lifecycle-stale_after
[§6]: https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md#6-cross-linking-and-paths
[§6.1]: https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md#61-links-between-concepts
[§6.2]: https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md#62-path-valued-fields
[§6.3]: https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md#63-the-references-convention
[§7]: https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md#7-actor-convention
[§8]: https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md#8-index-files
[§9]: https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md#9-log-files
[§10]: https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md#10-attested-computations-concept
[§10.3]: https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md#103-the-computation
[§11]: https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md#11-conformance
[§12]: https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md#12-versioning
