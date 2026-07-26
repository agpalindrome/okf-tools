# OKF friction log

Concrete places where the [OKF spec][spec] fights the tools built against it,
recorded with the date they surfaced (`CLAUDE.md`, "Deletion & creation"). This
is raw material for an eventual upstream conversation; whether and how to raise
any item is the owner's call.

[spec]: https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md

## 2026-07-26 — the actor convention (§7) excludes its own §5.1 example

[§7][s7] defines the actor convention as exactly three forms:

- `<producer>/<version>`, e.g. `reference_agent/gemini-2.5-pro`;
- `human:<id>`, e.g. `human:ahormati`;
- `process:<id>`, e.g. `process:finance-nightly`.

But [§5.1][s51]'s worked `sources` example writes `author: team:ga4-docs`, and
`team:…` is none of the three: it has no `/`, and its prefix is neither `human`
nor `process`. A checker that validates `author` / `by` strictly against §7's
list would flag the spec's own example.

**How okf-graph handles it.** Instead of the three-form whitelist, it accepts an
actor that is either `<producer>/<version>` (a `/` with non-empty sides) or
`<scheme>:<id>` (a `:` with a non-empty scheme and id). That admits `human:`,
`process:`, `team:`, and `producer/version`, and still rejects bare tokens like
`alice`. The permissiveness is deliberate: falsely rejecting a valid actor is a
defect reported against a conformant bundle, which is worse than accepting an
unusual-but-plausible one.

**The question for upstream.** Is §7's list meant to be exhaustive, or is the
real convention `<scheme>:<id>` | `<producer>/<version>`, with `human:` and
`process:` two sanctioned schemes among others such as `team:`? Either the list
should admit a general `<scheme>:<id>` form, or §5.1's example should change. The
`human:` prefix is load-bearing for §5.3 trust tiers, so its special status is
clear; what is unclear is whether other schemes are legal.

**Raised upstream** 2026-07-26 as
[GoogleCloudPlatform/knowledge-catalog#234][issue].

## 2026-07-26 — inline computation: §10.3 says fenced, §10.2 indents

[§10.3][s103] says an Attested Computation may carry its computation inline as
*"a single fenced code block in the body under `# Computation`."* But
[§10.2][s102]'s worked example puts the computation in a **4-space-indented**
code block, not a fence:

```markdown
# Computation
    SELECT SUM(amount) AS revenue
    FROM finance.recognized_revenue
    WHERE fiscal_year = @year
```

So a checker that detects the inline computation by looking for a fenced block
under `# Computation` would miss the spec's own example, and (for the §10.3
computation-XOR-fence rule) wrongly conclude the computation is neither inline
nor a `computation:` path.

**How okf-graph handles it.** It keys the XOR on the **presence of a
`# Computation` heading** in the body (scanned outside fenced code), not on the
code block's style — so both a fenced and an indented computation count as
inline. It does not yet inspect the block's contents (deferred, #58).

**The question for upstream.** Should §10.3 say "a fenced *or indented* code
block," or should §10.2's example be changed to a fenced block?

**Raised upstream** 2026-07-26 as
[GoogleCloudPlatform/knowledge-catalog#235][issue-235].

[issue]: https://github.com/GoogleCloudPlatform/knowledge-catalog/issues/234
[issue-235]: https://github.com/GoogleCloudPlatform/knowledge-catalog/issues/235
[s7]: https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md#7-actor-convention
[s51]: https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md#51-provenance-sources
[s102]: https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md#102-contract-fields
[s103]: https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md#103-the-computation
