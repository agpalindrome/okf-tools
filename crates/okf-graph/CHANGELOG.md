# Changelog

Notable changes to `okf-graph`, newest first. The crate is pre-1.0: a minor bump
may break the API, and an MSRV change is one of the things that earns it.

## 0.1.0 — 2026-08-08

The first release. Everything below is what the crate is, rather than a change
to something a reader already has.

### Added

- Bundle loading (`Bundle::load`): every non-reserved `.md` beneath a root
  becomes a concept keyed by its Concept ID, with the reserved `index.md` and
  `log.md` excluded from the concept set and validated as themselves.
- The concept-document model — a `Frontmatter` block and an opaque `Body` —
  reading the fields [SPEC][spec] §4.1 names and the §5 provenance, trust, and
  lifecycle families. A document that fails conformance still parses, so a
  finding can be located against it.
- 24 rules, each a located `Finding` carrying a defect or a tolerated report:
  document shape and `type` (§4, §11), the trust families and the actor
  convention (§5.2, §7), the credibility signals (§5.1), lifecycle `status` and
  `stale_after` (§5.4, §5.5), body-link and path resolution (§6), the derivation
  graph and its cycles (§5.1), `index.md` and `log.md` structure (§8, §9, §12),
  and the Attested Computation contract (§10).
- `Timestamp` and `Date`, which read §5's `at` fields as RFC 3339 and its
  `YYYY-MM-DD` fields as real calendar days. Both order chronologically, so a
  consumer compares values rather than the documents' own strings.
- An `okf-graph` binary: `okf-graph <bundle>` prints each finding and exits `0`
  with no defects, `1` with one or more, `2` on a usage or IO error.

### Compatibility

`Rule`, the frontmatter families, and the graph records are `#[non_exhaustive]`,
so match a wildcard arm and do not build them with a struct literal. New rules
and new spec fields then arrive as ordinary releases rather than as breaks —
which is the expected shape of the work, since both the checks and OKF itself
are still growing. `Severity` is exhaustive on purpose: it is §11's own binary,
not a list that grows.

[spec]: https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md
