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
}

impl Rule {
    /// Stable short code, e.g. `CONCEPT-1`.
    pub fn code(self) -> &'static str {
        match self {
            Rule::NotAConcept => "CONCEPT-1",
            Rule::MissingType => "CONCEPT-2",
            Rule::DuplicateId => "BUNDLE-1",
        }
    }

    /// Human-readable rule name.
    pub fn title(self) -> &'static str {
        match self {
            Rule::NotAConcept => "not a concept document",
            Rule::MissingType => "missing type",
            Rule::DuplicateId => "duplicate concept id",
        }
    }

    /// Whether this rule is a defect or a tolerated report.
    pub fn severity(self) -> Severity {
        match self {
            Rule::NotAConcept | Rule::MissingType | Rule::DuplicateId => Severity::Defect,
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

    const ALL: [Rule; 3] = [Rule::NotAConcept, Rule::MissingType, Rule::DuplicateId];

    #[test]
    fn every_rule_has_a_unique_code_and_a_title() {
        let mut codes = std::collections::BTreeSet::new();
        for rule in ALL {
            assert!(!rule.title().is_empty(), "{rule:?} has no title");
            assert!(codes.insert(rule.code()), "duplicate code {}", rule.code());
        }
    }

    #[test]
    fn the_base_rules_are_all_defects() {
        for rule in ALL {
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
