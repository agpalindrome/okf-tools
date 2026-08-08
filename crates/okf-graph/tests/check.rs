//! Acceptance tests for caller-supplied checks (issue #89): a caller's own
//! requirements, run over a loaded bundle, reported as findings okf-graph never
//! had to understand.

use std::path::PathBuf;

use okf_graph::{Bundle, Check, CheckError, Checks, Concept, Level, Policy, Rule, RuleId};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

/// The house rule #83 argued about, written where it belongs: in the caller.
/// §5.2 marks only `generated.by` required, so okf-graph does not ask for `at`.
struct GeneratedAtRequired;

impl Check for GeneratedAtRequired {
    fn code(&self) -> &str {
        "HOUSE-1"
    }
    fn check(&self, _id: &str, concept: &Concept) -> Result<(), String> {
        match concept.frontmatter().generated() {
            Some(generated) if generated.at.is_some() => Ok(()),
            Some(_) => Err("`generated` declares no `at`".into()),
            None => Err("no `generated` family".into()),
        }
    }
}

/// A check that informs rather than gates, to exercise `default_level`.
struct AlwaysReports;

impl Check for AlwaysReports {
    fn code(&self) -> &str {
        "HOUSE-2"
    }
    fn default_level(&self) -> Level {
        Level::Report
    }
    fn check(&self, id: &str, _concept: &Concept) -> Result<(), String> {
        Err(format!("saw `{id}`"))
    }
}

fn checks(items: Vec<Box<dyn Check>>) -> Checks {
    let mut checks = Checks::new();
    for item in items {
        checks.add(item).expect("registers");
    }
    checks
}

/// The whole point: a requirement the spec does not make, enforced by a caller
/// over a bundle okf-graph considers clean.
#[test]
fn a_caller_rule_fires_on_a_bundle_the_spec_calls_clean() {
    let bundle = Bundle::load(&fixture("dangling")).expect("loads");
    assert!(!bundle.fails(&Policy::new()), "spec-clean but for a report");

    let checks = checks(vec![Box::new(GeneratedAtRequired)]);
    let findings = bundle.check(&checks);

    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].rule, RuleId::Custom("HOUSE-1".into()));
    assert_eq!(findings[0].file, "note.md");
}

/// A caller finding has no `Severity`: §11 has no verdict on a house rule, and
/// a defaulted one would put words in the spec's mouth.
#[test]
fn a_caller_finding_has_no_severity_but_does_have_a_level() {
    let bundle = Bundle::load(&fixture("dangling")).expect("loads");
    let checks = checks(vec![Box::new(GeneratedAtRequired)]);
    let findings = bundle.check(&checks);

    assert_eq!(findings[0].severity(), None);
    assert_eq!(findings[0].rule.spec(), None);
    assert_eq!(
        Policy::for_checks(&checks).level(&findings[0].rule),
        Level::Defect
    );
}

/// Display tells the two apart on sight: a spec finding names its rule, a
/// caller's prints the code alone.
#[test]
fn the_two_kinds_of_finding_print_differently() {
    let bundle = Bundle::load(&fixture("dangling")).expect("loads");
    let checks = checks(vec![Box::new(GeneratedAtRequired)]);

    assert_eq!(
        bundle.findings()[0].to_string(),
        "note.md\tBUNDLE-2 (dangling link): link to `/tables/ghost.md` resolves to no concept in the bundle"
    );
    assert_eq!(
        bundle.check(&checks)[0].to_string(),
        "note.md\tHOUSE-1: no `generated` family"
    );
}

/// A check's own `default_level` reaches the policy through `for_checks`, so a
/// caller does not restate what it already wrote on the check.
#[test]
fn a_checks_default_level_seeds_the_policy() {
    let checks = checks(vec![Box::new(GeneratedAtRequired), Box::new(AlwaysReports)]);
    let policy = Policy::for_checks(&checks);

    assert_eq!(
        policy.level(&RuleId::Custom("HOUSE-1".into())),
        Level::Defect
    );
    assert_eq!(
        policy.level(&RuleId::Custom("HOUSE-2".into())),
        Level::Report
    );
    // Untouched: seeding caller rules says nothing about the spec's.
    assert_eq!(
        policy.level(&RuleId::Spec(Rule::DanglingLink)),
        Level::Report
    );
}

/// A caller's rule nothing has set falls back to `Defect` — the check fired,
/// and swallowing it would be the worse guess.
#[test]
fn an_unseeded_caller_rule_defaults_to_defect() {
    assert_eq!(
        Policy::new().level(&RuleId::Custom("NEVER-SEEN".into())),
        Level::Defect
    );
}

/// A caller's rule takes levels like any other, including being silenced.
#[test]
fn a_caller_rule_can_be_denied_warned_or_allowed() {
    let mut policy = Policy::for_checks(&checks(vec![Box::new(GeneratedAtRequired)]));
    let rule = RuleId::Custom("HOUSE-1".into());

    assert_eq!(policy.level(&rule), Level::Defect);
    policy.set(rule.clone(), Level::Allow);
    assert_eq!(policy.level(&rule), Level::Allow);
}

/// An unmatched level is inert and askable, not an error: one policy shared
/// across several check sets legitimately names rules absent from this run.
#[test]
fn unmatched_reports_codes_no_check_registers() {
    let checks = checks(vec![Box::new(GeneratedAtRequired)]);
    let mut policy = Policy::for_checks(&checks);
    policy.set(RuleId::Custom("HOUSE-9".into()), Level::Allow);
    policy.set(Rule::DanglingLink, Level::Defect);

    assert_eq!(policy.unmatched(&checks), ["HOUSE-9"]);
}

#[test]
fn a_code_collision_is_refused_at_registration() {
    let mut checks = Checks::new();
    checks.add(GeneratedAtRequired).expect("registers");

    assert_eq!(
        checks.add(GeneratedAtRequired),
        Err(CheckError::DuplicateCode("HOUSE-1".into()))
    );
    assert_eq!(checks.len(), 1);
}

/// A bundle with no concepts runs no caller checks — which is #90's problem,
/// recorded here so the behaviour is at least pinned.
#[test]
fn no_concepts_means_no_caller_findings() {
    let dir = std::env::temp_dir().join("okf-graph-check-empty");
    std::fs::create_dir_all(&dir).expect("mkdir");
    let bundle = Bundle::load(&dir).expect("loads");

    assert!(bundle.is_empty());
    assert!(bundle
        .check(&checks(vec![Box::new(AlwaysReports)]))
        .is_empty());
}
