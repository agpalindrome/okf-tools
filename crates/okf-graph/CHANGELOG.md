# Changelog

Notable changes to `okf-graph`, newest first. The crate is pre-1.0: a minor bump
may break the API, and an MSRV change is one of the things that earns it.

## Unreleased

### Added

- **Caller-supplied checks**: a `Check` trait, a `Checks` set that verifies codes
  are unique, and `Bundle::check`. A caller writes its own requirements — a house
  key in every frontmatter, a `generated.at` §5.2 does not require — and
  okf-graph runs them without learning what they are. The crate extends exactly
  as far as the OKF spec and no further, so a house rule attaches here rather
  than being argued into the rule set. A check's findings carry a
  `RuleId::Custom` and take `Level`s like any other, and `Policy::for_checks`
  seeds each one from the `default_level` the check declares. Named in
  [#89](https://github.com/ojhermann-org/okf-tools/issues/89).
- Rule **levels** a consumer sets: `Level` (`Allow` / `Report` / `Defect`),
  `Policy`, `Rule::default_level`, `Rule::from_code`, `Rule::ALL`, and
  `Bundle::findings_at` / `Bundle::fails`. The binary takes `--deny`, `--warn`,
  and `--allow` over a rule code, last-wins, and counts what `--allow` dropped.
  §11's tolerance is the right default for a consumer reading somebody else's
  bundle and the wrong policy for a producer checking its own, so `--deny
  BUNDLE-2` now fails on a dangling link without touching any other tolerated
  rule. `Severity` is untouched and stays the spec's own verdict — a level is a
  consumer's decision layered over it, never a re-reading of §11. Defaults come
  from `Severity`, so a run that sets nothing behaves exactly as before. Named
  in [#83](https://github.com/ojhermann-org/okf-tools/issues/83); the producer
  rules that issue also proposes are not here.
- `Frontmatter::declares`, `Frontmatter::scalar`, and
  `Frontmatter::executor_receipt_malformed` are public. The `Some`-only-on-shape
  readers conflate "absent" with "present but the wrong shape" on purpose, and
  the methods that told them apart were `pub(crate)` — so a check inside the
  crate could make the distinction the type is designed around and a consumer
  could not. `declares` answers whether the key was written at all; `scalar`
  renders the value as the document wrote it, which separates `status:
  provisional` (an unmodelled lifecycle stage) from `status: true` (a value
  nothing can read) where `declares` alone reports both alike. Named in
  [#84](https://github.com/ojhermann-org/okf-tools/issues/84).

### Changed

- **Breaking:** `Finding.rule` is a `RuleId` rather than a `Rule` — either
  `Spec(Rule)`, the OKF list, or `Custom(Arc<str>)`, a caller's own code (#91).
  `finding.rule == Rule::MissingType` still compiles, via `PartialEq<Rule>`.
- **Breaking:** `Finding::severity` returns `Option<Severity>`. §11 has a verdict
  on an OKF rule and none on a house rule, so a caller finding has no severity
  rather than a defaulted one that would put words in the spec's mouth. What it
  has instead is a `Level`, which is the question an exit code should be asking.
- **Breaking:** `Policy::level` takes `&RuleId`, and `Policy::set` takes anything
  `Into<RuleId>` — so existing `policy.set(Rule::DanglingLink, …)` is unchanged.
- The licence is `MIT OR Apache-2.0` rather than Apache-2.0 alone, so a consumer
  chooses rather than inheriting the stricter of the two. 0.1.0 on crates.io
  stays Apache-2.0 — a published version is immutable — so the pair reaches the
  registry only with the next release.
- `NOTICE` is gone. Under a dual licence its text was untrue for anyone electing
  MIT, and keeping it would have handed Apache-2.0 §4(d)'s propagation duty to
  every redistributor for no gain. Copyright attribution lives in `LICENSE-MIT`.

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
