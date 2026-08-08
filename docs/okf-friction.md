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

## 2026-07-26 — a scope descriptor is indistinguishable from a path

[§5.1][s51] says a `sources[].resource` is *either* a followable path (a URL, a
bundle-relative path, or a `references/…` path) *or* "a population or scope
descriptor it cannot [follow]" — `all queries in BigQuery project X`. [§6.2][s62]
restates it: "in which case it is not a path." But neither gives a consumer a
syntactic way to tell the two apart, so a tool that resolves `sources` and
flags a `resource` pointing nowhere has to guess which it is holding.

**How okf-graph handles it.** It classifies a `sources[].resource` as a scope
descriptor when it is non-URL, non-`/`-rooted, and **contains a space** — natural
language rather than a path. Fragile both ways (a single-token scope, or a path
with a `%20`/space), but it errs toward *not* reporting a dangling path, so a
scope descriptor is never mistaken for a broken one.

**The question for upstream.** Should a scope descriptor carry a marker (a
distinct key, or a structured `resource: { scope: … }`) so tooling stops
inferring intent from punctuation?

**Raised upstream** 2026-07-26 as
[GoogleCloudPlatform/knowledge-catalog#236][issue-236].

## 2026-07-27 — §9 states "newest first" without marking its normative force

[§9][s9] describes a `log.md` as "a flat list of date-grouped entries, **newest
first**," then says date headings **MUST** use ISO 8601, and that the bold lead
word is "a convention, **not a requirement**." Two of the three statements are
explicitly marked — one a MUST, one a convention — and the ordering ("newest
first") is marked as neither. So a checker cannot tell whether an out-of-order
log is non-conformant (§11 requires reserved files follow §9) or merely
untidy.

**How okf-graph handles it.** A non-ISO date heading is a defect (the explicit
§9 MUST); an out-of-order log is a **report**, not a defect — surfaced, but not
failed, since §9 does not mark ordering as required. This follows the standing
rule of not failing a bundle the spec does not clearly make non-conformant.

**Not raised upstream** (2026-07-27). This is the fourth instance of one
pattern — the spec stating a constraint without marking its normative force
(cf. [#234][issue], [#235][issue-235], [#236][issue-236] for actors, the inline
computation, and scope descriptors). Held to raise as a single pattern
observation once the working group engages the open issues, rather than filing
a fourth in silence.

## 2026-07-28 — §11 conforms over "the tree", which is never defined

[§11][s11] makes a bundle conformant when "every non-reserved `.md` file in the
tree" parses, but nothing in v0.2 says what "the tree" is: where it is rooted,
whether a nested bundle inside it is one corpus or two, and — the case that
bites an implementation — whether a symlink entry is a member of it.

That last one is not hypothetical. Reading symlinks as documents gives one file
two Concept IDs, and a directory symlink pointing at an ancestor re-walks the
whole tree once per link the platform will resolve: a single-concept bundle
loaded as 33 concepts on macOS, which stops there only because the OS refuses a
34th resolution. The corpus size becomes a property of the platform.

**How okf-graph handles it.** A symlink entry is not a document: it is never
parsed as a concept, never checked as a reserved file, and never descended
into. It is still recorded in the file set, so a path-valued field
([§6.2][s62]) or an index entry naming one resolves rather than dangling — the
don't-fail-what-the-spec-doesn't-forbid rule the other entries here follow,
and consistent with dereferencing being consumer policy.

**Not raised upstream** (2026-07-28), because upstream is already fixing it:
open PR [#232][pr232] adds a §3.2 defining a single bundle root and a
conformance corpus of "regular `.md` file entries recursively beneath that
root, excluding symbolic-link entries", and rewrites §11's list in those terms.
Our behaviour was chosen independently and matches it. Logged as a dated record
of the ambiguity, not as something to file; if #232 stalls, this is the
evidence for asking.

## 2026-08-08 — "an ISO 8601 datetime" (§5.2) admits a form that sorts backwards

[§5.2][s52] types `generated.at` and `verified[].at` as "an ISO 8601 datetime"
and narrows it no further. ISO 8601 is a family of formats, and the week date is
the member that bites: `2026-W01-1T00:00:00Z` denotes 2025-12-29, yet it is the
same length as the calendar form, carries the same separators at the same
offsets, and sorts *after* every calendar date, because `W` exceeds every digit.

So a consumer comparing the two fields as strings — the obvious implementation,
since both arrive as strings — reads a week-dated verification as newer than any
calendar-dated regeneration. The input is legal, so nothing warns. That is not
hypothetical: a downstream bundle's staleness check pinned
the format by testing the length, the trailing `Z`, and the separators at
offsets 4 and 10, and a week date satisfies all four (issue #72).

**How okf-graph handles it.** It reads `at` as RFC 3339 and reports `CONCEPT-12`
when it will not parse — a defect, because §5.2 states the type rather than
suggesting it, and §11's list of what a consumer must tolerate does not reach a
malformed one. The narrowing costs a conformant author nothing: all 15 `at`
values in v0.2's examples are `YYYY-MM-DDTHH:MM:SSZ`, which RFC 3339 admits.
This is the one entry here where okf-graph is *stricter* than the text rather
than more permissive, so it is the one to revisit first if the working group
answers otherwise.

**The question for upstream.** Does §5.2 mean the RFC 3339 profile its every
example uses, or ISO 8601 entire? The looser reading obliges a consumer to
handle week dates, ordinal dates, and the basic format, none of which appear
anywhere in the spec. It also makes the fields unsafe to order, which is what
comparing `verified` against `generated` exists to do. **Not raised upstream**
(2026-08-08); raising it is the owner's call.

[pr232]: https://github.com/GoogleCloudPlatform/knowledge-catalog/pull/232
[s11]: https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md#11-conformance
[issue]: https://github.com/GoogleCloudPlatform/knowledge-catalog/issues/234
[issue-235]: https://github.com/GoogleCloudPlatform/knowledge-catalog/issues/235
[issue-236]: https://github.com/GoogleCloudPlatform/knowledge-catalog/issues/236
[s62]: https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md#62-path-valued-fields
[s7]: https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md#7-actor-convention
[s51]: https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md#51-provenance-sources
[s52]: https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md#52-trust-generated-and-verified
[s102]: https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md#102-contract-fields
[s103]: https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md#103-the-computation
[s9]: https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md#9-log-files
