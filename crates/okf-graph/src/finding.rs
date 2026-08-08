//! Located findings and the severity vocabulary every check reports in.
//! Modelled on `deon`'s `Finding` / `Severity` / `Rule` without depending on the
//! archived crate; the rationale is `docs/okf-graph-DESIGN.md` §5.

use std::fmt;
use std::sync::Arc;

/// Whether a finding is a **defect** to fix or a **report** about something the
/// spec says to tolerate.
///
/// The split is the spec's: a dangling link or a missing optional family MUST
/// NOT be rejected (SPEC §6, §11), so those are reports — printed, but they do
/// not fail a run. Collapsing them into defects would fail a conformant bundle.
///
/// Deliberately **not** `#[non_exhaustive]`, unlike [`Rule`] and the frontmatter
/// families. Those grow as checks and the spec do; this is §11's own binary —
/// a bundle either conforms or it does not — so a caller matching both arms has
/// covered the domain, and warning them otherwise would be false.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// A conformance failure to fix.
    Defect,
    /// Something the spec says to tolerate; surfaced, but does not fail a run.
    Report,
}

/// Every rule a check can report. Codes follow the two levels of
/// `docs/okf-graph-DESIGN.md` §6: `CONCEPT-*` reads one document, `BUNDLE-*`
/// reads the graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Rule {
    /// CONCEPT-1: a non-reserved `.md` that does not parse as a concept (§11).
    NotAConcept,
    /// CONCEPT-2: a concept with no non-empty `type` (§11).
    MissingType,
    /// BUNDLE-1: two concept files resolve to the same Concept ID (§2).
    DuplicateId,
    /// BUNDLE-2: a body link resolves to no concept in the bundle (§6). A
    /// report, not a defect — the spec says a broken link may just be
    /// not-yet-written knowledge.
    DanglingLink,
    /// CONCEPT-3: `status` is present but not `draft` / `stable` / `deprecated`
    /// (§5.4).
    InvalidStatus,
    /// CONCEPT-4: a declared `generated` block carries no `by` (§5.2 REQUIRED).
    MissingGeneratedBy,
    /// CONCEPT-5: an actor field (`generated.by`, `verified[].by`, or a source
    /// `author`) does not match the §7 convention.
    MalformedActor,
    /// CONCEPT-6: a `sources` entry declares no `resource` (§5.1 REQUIRED).
    MissingSourceResource,
    /// CONCEPT-7: an `Attested Computation` concept declares no `runtime`
    /// (§10.2 REQUIRED for the type).
    MissingRuntime,
    /// CONCEPT-8: an `Attested Computation` gives its computation in neither or
    /// both of the two §10.3 places (a `computation:` path xor a `# Computation`
    /// body block) — it must be exactly one.
    InvalidComputationSource,
    /// BUNDLE-3: a path-valued field (§6.2) names a bundle file that is not
    /// there. A report, not a defect — §6/§11 tolerate a broken reference.
    DanglingPath,
    /// BUNDLE-4: the derivation edges (§5.1) form a cycle. A report — §11 is
    /// silent on acyclicity, and telling a benign loop from a genuine
    /// contradiction is deferred (#62).
    DerivationCycle,
    /// INDEX-1: an `index.md` carries frontmatter it may not — any in a nested
    /// index, or a key other than `okf_version` at the root (§8/§12). A defect:
    /// §11 requires reserved files to follow §8's structure.
    IndexFrontmatter,
    /// INDEX-2: an `index.md` entry link resolves to no concept, file, or
    /// subdirectory (§8). A report — a broken link is tolerated (§6).
    DanglingIndexEntry,
    /// INDEX-3: a root `index.md` declares an `okf_version` this tool does not
    /// understand (§12). A report — §12 says best-effort, do not refuse.
    UnknownOkfVersion,
    /// LOG-1: a `log.md` date heading is not ISO-8601 `YYYY-MM-DD` (§9). A
    /// defect: §9 makes the date format an explicit MUST.
    NonIsoLogDate,
    /// LOG-2: a `log.md`'s date headings are not in newest-first order (§9). A
    /// report — §9 states the order but does not mark it a MUST (#47 friction).
    LogOutOfOrder,
    /// LOG-3: a `log.md` entry link resolves to no concept, file, or directory
    /// (§9). A report — a broken link is tolerated (§6).
    DanglingLogEntry,
    /// CONCEPT-9: an Attested Computation `parameters` entry is missing `name`,
    /// `type`, or a boolean `required` (§10.2). A report — parameters are an
    /// optional supporting field, not §10's required core.
    MalformedParameter,
    /// CONCEPT-10: an Attested Computation's `executor` / `attester` is
    /// incomplete — no `resource`, or an `executor.receipt` that is not a list
    /// (§10.2). A report — the attestation machinery is a supporting field.
    IncompleteAttestation,
    /// CONCEPT-11: an Attested Computation's `# Computation` section is empty
    /// and there is no `computation:` path — the inline source is declared but
    /// delivers nothing (§10.3). A defect: like CONCEPT-8, the computation is
    /// the required core, and this one has none.
    EmptyComputation,
    /// CONCEPT-12: a `generated.at` or `verified[].at` that is not an RFC 3339
    /// datetime (§5.2). A defect: §5.2 states the type rather than suggesting
    /// it, and §11's list of what a consumer must tolerate does not reach a
    /// malformed one.
    MalformedTimestamp,
    /// CONCEPT-13: `stale_after` is present but not a `YYYY-MM-DD` date (§5.5).
    /// A defect, on the same reading as CONCEPT-12: §5.5 states the format, and
    /// a staleness decision a consumer cannot make is worse than an absent one.
    MalformedStaleAfter,
    /// CONCEPT-14: a §5.1 credibility signal is present but unusable — a
    /// `last_modified` or `usage_window` bound that is not `YYYY-MM-DD`, or a
    /// `usage_count` that is not an integer. A report, matching CONCEPT-9 and
    /// CONCEPT-10: these are supporting signals a consumer weighs, not a
    /// required core it needs.
    MalformedSourceSignal,
}

impl Rule {
    /// Every rule. A slice rather than an array, so adding a rule does not
    /// change the type a caller wrote down.
    ///
    /// The `#[non_exhaustive]` seal stops a consumer matching every variant; it
    /// was never meant to stop one *listing* them, and naming a rule on a
    /// command line needs the list.
    pub const ALL: &'static [Rule] = &[
        Rule::NotAConcept,
        Rule::MissingType,
        Rule::DuplicateId,
        Rule::DanglingLink,
        Rule::InvalidStatus,
        Rule::MissingGeneratedBy,
        Rule::MalformedActor,
        Rule::MissingSourceResource,
        Rule::MissingRuntime,
        Rule::InvalidComputationSource,
        Rule::DanglingPath,
        Rule::DerivationCycle,
        Rule::IndexFrontmatter,
        Rule::DanglingIndexEntry,
        Rule::UnknownOkfVersion,
        Rule::NonIsoLogDate,
        Rule::LogOutOfOrder,
        Rule::DanglingLogEntry,
        Rule::MalformedParameter,
        Rule::IncompleteAttestation,
        Rule::EmptyComputation,
        Rule::MalformedTimestamp,
        Rule::MalformedStaleAfter,
        Rule::MalformedSourceSignal,
    ];

    /// The rule with this code, ASCII-case-insensitively — a code is typed by
    /// hand, and `bundle-2` means what `BUNDLE-2` means.
    ///
    /// `None` for a code no rule has. A caller reports that rather than
    /// ignoring it: a mistyped code that silently does nothing is worse than
    /// having no way to name a rule at all, because it looks like it worked.
    pub fn from_code(code: &str) -> Option<Rule> {
        Rule::ALL
            .iter()
            .copied()
            .find(|rule| rule.code().eq_ignore_ascii_case(code))
    }

    /// Stable short code, e.g. `CONCEPT-1`.
    pub fn code(self) -> &'static str {
        match self {
            Rule::NotAConcept => "CONCEPT-1",
            Rule::MissingType => "CONCEPT-2",
            Rule::DuplicateId => "BUNDLE-1",
            Rule::DanglingLink => "BUNDLE-2",
            Rule::InvalidStatus => "CONCEPT-3",
            Rule::MissingGeneratedBy => "CONCEPT-4",
            Rule::MalformedActor => "CONCEPT-5",
            Rule::MissingSourceResource => "CONCEPT-6",
            Rule::MissingRuntime => "CONCEPT-7",
            Rule::InvalidComputationSource => "CONCEPT-8",
            Rule::DanglingPath => "BUNDLE-3",
            Rule::DerivationCycle => "BUNDLE-4",
            Rule::IndexFrontmatter => "INDEX-1",
            Rule::DanglingIndexEntry => "INDEX-2",
            Rule::UnknownOkfVersion => "INDEX-3",
            Rule::NonIsoLogDate => "LOG-1",
            Rule::LogOutOfOrder => "LOG-2",
            Rule::DanglingLogEntry => "LOG-3",
            Rule::MalformedParameter => "CONCEPT-9",
            Rule::IncompleteAttestation => "CONCEPT-10",
            Rule::EmptyComputation => "CONCEPT-11",
            Rule::MalformedTimestamp => "CONCEPT-12",
            Rule::MalformedStaleAfter => "CONCEPT-13",
            Rule::MalformedSourceSignal => "CONCEPT-14",
        }
    }

    /// Human-readable rule name.
    pub fn title(self) -> &'static str {
        match self {
            Rule::NotAConcept => "not a concept document",
            Rule::MissingType => "missing type",
            Rule::DuplicateId => "duplicate concept id",
            Rule::DanglingLink => "dangling link",
            Rule::InvalidStatus => "invalid status",
            Rule::MissingGeneratedBy => "missing generated.by",
            Rule::MalformedActor => "malformed actor",
            Rule::MissingSourceResource => "missing source resource",
            Rule::MissingRuntime => "missing runtime",
            Rule::InvalidComputationSource => "invalid computation source",
            Rule::DanglingPath => "dangling path",
            Rule::DerivationCycle => "derivation cycle",
            Rule::IndexFrontmatter => "index frontmatter not allowed",
            Rule::DanglingIndexEntry => "dangling index entry",
            Rule::UnknownOkfVersion => "unknown okf_version",
            Rule::NonIsoLogDate => "non-ISO log date",
            Rule::LogOutOfOrder => "log out of order",
            Rule::DanglingLogEntry => "dangling log entry",
            Rule::MalformedParameter => "malformed parameter",
            Rule::IncompleteAttestation => "incomplete attestation contract",
            Rule::EmptyComputation => "empty computation",
            Rule::MalformedTimestamp => "malformed timestamp",
            Rule::MalformedStaleAfter => "malformed stale_after",
            Rule::MalformedSourceSignal => "malformed source signal",
        }
    }

    /// Whether this rule is a defect or a tolerated report.
    pub fn severity(self) -> Severity {
        match self {
            Rule::DanglingLink
            | Rule::DanglingPath
            | Rule::DerivationCycle
            | Rule::DanglingIndexEntry
            | Rule::UnknownOkfVersion
            | Rule::LogOutOfOrder
            | Rule::DanglingLogEntry
            | Rule::MalformedParameter
            | Rule::IncompleteAttestation
            | Rule::MalformedSourceSignal => Severity::Report,
            Rule::NotAConcept
            | Rule::MissingType
            | Rule::DuplicateId
            | Rule::InvalidStatus
            | Rule::MissingGeneratedBy
            | Rule::MalformedActor
            | Rule::MissingSourceResource
            | Rule::MissingRuntime
            | Rule::InvalidComputationSource
            | Rule::IndexFrontmatter
            | Rule::NonIsoLogDate
            | Rule::EmptyComputation
            | Rule::MalformedTimestamp
            | Rule::MalformedStaleAfter => Severity::Defect,
        }
    }
}

/// Which rule a finding is about: one of the spec's, or one a caller brought.
///
/// The two halves are deliberately distinguishable. [`Rule`] is and stays the
/// OKF list — sealed, complete, and the crate's whole remit. A caller's rule is
/// a house rule this crate knows nothing about beyond its code, and a reader of
/// the output should be able to tell which kind fired without consulting
/// anything.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RuleId {
    /// A rule from the OKF spec.
    Spec(Rule),
    /// A caller's own rule, named by its code. `Arc<str>` rather than
    /// `&'static str` because a caller may compute a code — one rule per
    /// required frontmatter key is the shape that motivated caller checks in the
    /// first place — and rather than `String` because a `Finding` is cloned per
    /// concept over a whole bundle (#91).
    Custom(Arc<str>),
}

impl RuleId {
    /// The rule's code: the spec's own, or the caller's.
    pub fn code(&self) -> &str {
        match self {
            RuleId::Spec(rule) => rule.code(),
            RuleId::Custom(code) => code,
        }
    }

    /// The spec rule this names, or `None` for a caller's.
    pub fn spec(&self) -> Option<Rule> {
        match self {
            RuleId::Spec(rule) => Some(*rule),
            RuleId::Custom(_) => None,
        }
    }
}

impl From<Rule> for RuleId {
    fn from(rule: Rule) -> RuleId {
        RuleId::Spec(rule)
    }
}

/// `finding.rule == Rule::MissingType` reads the obvious way, without the caller
/// wrapping the right-hand side or matching to get at it. A caller's rule is
/// never equal to a spec rule, which is the answer that keeps the seam honest.
impl PartialEq<Rule> for RuleId {
    fn eq(&self, other: &Rule) -> bool {
        matches!(self, RuleId::Spec(rule) if rule == other)
    }
}

impl PartialEq<RuleId> for Rule {
    fn eq(&self, other: &RuleId) -> bool {
        other == self
    }
}

/// A located finding: the file it is about, the rule, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Finding {
    /// Bundle-relative path of the file the finding is about, e.g.
    /// `tables/orders.md`. A file rather than a Concept ID, because a file that
    /// fails to parse (`CONCEPT-1`) has no id to name.
    pub file: String,
    /// Which rule tripped.
    pub rule: RuleId,
    /// One-line explanation.
    pub detail: String,
}

impl Finding {
    pub(crate) fn new(
        file: impl Into<String>,
        rule: impl Into<RuleId>,
        detail: impl Into<String>,
    ) -> Self {
        Finding {
            file: file.into(),
            rule: rule.into(),
            detail: detail.into(),
        }
    }

    /// The finding's severity, or `None` when a caller's rule tripped.
    ///
    /// `Severity` is §11's verdict on a bundle, and §11 has no opinion about a
    /// house rule — so a caller finding has no severity rather than a defaulted
    /// one. What it has instead is a [`Level`], which is the consumer's own
    /// question and the one an exit code should be reading.
    ///
    /// [`Level`]: crate::Level
    pub fn severity(&self) -> Option<Severity> {
        self.rule.spec().map(Rule::severity)
    }
}

impl fmt::Display for Finding {
    /// A spec finding names its rule, `path CONCEPT-2 (missing type): …`; a
    /// caller's prints the code alone, `path TV-1: …`. The asymmetry is the
    /// point — the crate has no title for a rule it did not write, and the
    /// shorter line tells a reader at a glance whose rule fired.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.rule {
            RuleId::Spec(rule) => write!(
                f,
                "{}\t{} ({}): {}",
                self.file,
                rule.code(),
                rule.title(),
                self.detail
            ),
            RuleId::Custom(code) => write!(f, "{}\t{}: {}", self.file, code, self.detail),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_rule_has_a_unique_code_and_a_title() {
        let mut codes = std::collections::BTreeSet::new();
        for rule in Rule::ALL.iter().copied() {
            assert!(!rule.title().is_empty(), "{rule:?} has no title");
            assert!(codes.insert(rule.code()), "duplicate code {}", rule.code());
        }
    }

    /// The load-bearing property of `ALL`: a rule left out of it is unreachable
    /// by code, and this is what says so. `severity()`'s match is exhaustive, so
    /// the compiler already forces a new rule to declare one — the list is the
    /// part nothing else checks.
    #[test]
    fn every_rule_round_trips_through_its_code() {
        for rule in Rule::ALL.iter().copied() {
            assert_eq!(Rule::from_code(rule.code()), Some(rule), "{rule:?}");
        }
        assert_eq!(
            Rule::ALL.len(),
            24,
            "a rule was added without a test update"
        );
    }

    #[test]
    fn a_code_reads_in_any_case_and_an_unknown_one_is_none() {
        assert_eq!(Rule::from_code("bundle-2"), Some(Rule::DanglingLink));
        assert_eq!(Rule::from_code("BUNDLE-2"), Some(Rule::DanglingLink));
        assert_eq!(Rule::from_code("BUNDEL-2"), None);
        assert_eq!(Rule::from_code(""), None);
    }

    #[test]
    fn severity_splits_the_tolerated_report_from_the_defects() {
        assert_eq!(Rule::DanglingLink.severity(), Severity::Report);
        assert_eq!(Rule::DanglingPath.severity(), Severity::Report);
        assert_eq!(Rule::DerivationCycle.severity(), Severity::Report);
        assert_eq!(Rule::DanglingIndexEntry.severity(), Severity::Report);
        assert_eq!(Rule::UnknownOkfVersion.severity(), Severity::Report);
        assert_eq!(Rule::LogOutOfOrder.severity(), Severity::Report);
        assert_eq!(Rule::DanglingLogEntry.severity(), Severity::Report);
        assert_eq!(Rule::MalformedParameter.severity(), Severity::Report);
        assert_eq!(Rule::IncompleteAttestation.severity(), Severity::Report);
        assert_eq!(Rule::MalformedSourceSignal.severity(), Severity::Report);
        for rule in [
            Rule::NotAConcept,
            Rule::MissingType,
            Rule::DuplicateId,
            Rule::InvalidStatus,
            Rule::MissingGeneratedBy,
            Rule::MalformedActor,
            Rule::MissingSourceResource,
            Rule::MissingRuntime,
            Rule::InvalidComputationSource,
            Rule::IndexFrontmatter,
            Rule::NonIsoLogDate,
            Rule::EmptyComputation,
            Rule::MalformedTimestamp,
            Rule::MalformedStaleAfter,
        ] {
            assert_eq!(rule.severity(), Severity::Defect, "{rule:?}");
        }
    }

    #[test]
    fn display_locates_the_file_then_names_the_rule() {
        let finding = Finding::new("tables/orders.md", Rule::MissingType, "no `type`");
        assert_eq!(
            finding.to_string(),
            "tables/orders.md\tCONCEPT-2 (missing type): no `type`"
        );
        assert_eq!(finding.severity(), Some(Severity::Defect));
    }
}
