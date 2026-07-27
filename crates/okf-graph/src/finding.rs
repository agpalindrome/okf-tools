//! Located findings and the severity vocabulary every check reports in.
//! Modelled on `deon`'s `Finding` / `Severity` / `Rule` without depending on the
//! archived crate; the rationale is `docs/okf-graph-DESIGN.md` §5.

use std::fmt;

/// Whether a finding is a **defect** to fix or a **report** about something the
/// spec says to tolerate.
///
/// The split is the spec's: a dangling link or a missing optional family MUST
/// NOT be rejected (SPEC §6, §11), so those are reports — printed, but they do
/// not fail a run. Collapsing them into defects would fail a conformant bundle.
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
}

impl Rule {
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
        }
    }

    /// Whether this rule is a defect or a tolerated report.
    pub fn severity(self) -> Severity {
        match self {
            Rule::DanglingLink
            | Rule::DanglingPath
            | Rule::DerivationCycle
            | Rule::DanglingIndexEntry
            | Rule::UnknownOkfVersion => Severity::Report,
            Rule::NotAConcept
            | Rule::MissingType
            | Rule::DuplicateId
            | Rule::InvalidStatus
            | Rule::MissingGeneratedBy
            | Rule::MalformedActor
            | Rule::MissingSourceResource
            | Rule::MissingRuntime
            | Rule::InvalidComputationSource
            | Rule::IndexFrontmatter => Severity::Defect,
        }
    }
}

/// A located finding: the file it is about, the rule, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// Bundle-relative path of the file the finding is about, e.g.
    /// `tables/orders.md`. A file rather than a Concept ID, because a file that
    /// fails to parse (`CONCEPT-1`) has no id to name.
    pub file: String,
    /// Which rule tripped.
    pub rule: Rule,
    /// One-line explanation.
    pub detail: String,
}

impl Finding {
    pub(crate) fn new(file: impl Into<String>, rule: Rule, detail: impl Into<String>) -> Self {
        Finding {
            file: file.into(),
            rule,
            detail: detail.into(),
        }
    }

    /// The finding's severity, from its rule.
    pub fn severity(&self) -> Severity {
        self.rule.severity()
    }
}

impl fmt::Display for Finding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}\t{} ({}): {}",
            self.file,
            self.rule.code(),
            self.rule.title(),
            self.detail
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [Rule; 15] = [
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
    ];

    #[test]
    fn every_rule_has_a_unique_code_and_a_title() {
        let mut codes = std::collections::BTreeSet::new();
        for rule in ALL {
            assert!(!rule.title().is_empty(), "{rule:?} has no title");
            assert!(codes.insert(rule.code()), "duplicate code {}", rule.code());
        }
    }

    #[test]
    fn severity_splits_the_tolerated_report_from_the_defects() {
        assert_eq!(Rule::DanglingLink.severity(), Severity::Report);
        assert_eq!(Rule::DanglingPath.severity(), Severity::Report);
        assert_eq!(Rule::DerivationCycle.severity(), Severity::Report);
        assert_eq!(Rule::DanglingIndexEntry.severity(), Severity::Report);
        assert_eq!(Rule::UnknownOkfVersion.severity(), Severity::Report);
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
        assert_eq!(finding.severity(), Severity::Defect);
    }
}
