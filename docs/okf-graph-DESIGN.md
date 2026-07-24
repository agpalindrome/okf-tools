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
  another, never _what_ the relationship means. OKF links are untyped (§6), and
  the meaning of a norm — obligation, defeat, the mechanical/judgment seam — is
  `okf-normative`'s to read, one layer up.

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

[okf-spec]: https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md
[board]: https://github.com/orgs/ojhermann-org/projects/8
