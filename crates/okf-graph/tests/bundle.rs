//! Acceptance tests for `Bundle::load` (issue #40): identity and reserved-file
//! exclusion over a clean multi-directory bundle.

use std::path::PathBuf;

use okf_graph::{Bundle, Rule, Severity};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

/// Every non-reserved `.md` is a concept keyed by its bundle-relative path
/// minus `.md`; nested directories become `/`-joined ids. The reserved
/// `index.md` / `log.md` at either level are not concepts.
#[test]
fn loads_concepts_by_id_and_excludes_reserved_files() {
    let bundle = Bundle::load(&fixture("clean")).expect("clean bundle loads");

    let ids: Vec<&str> = bundle.concepts().map(|(id, _)| id).collect();
    assert_eq!(ids, ["overview", "tables/customers", "tables/orders"]);
    assert_eq!(bundle.len(), 3);

    assert!(bundle.concept("index").is_none());
    assert!(bundle.concept("log").is_none());
    assert!(bundle.concept("tables/index").is_none());
}

/// A loaded concept carries the frontmatter and body `Concept::parse` read.
#[test]
fn a_loaded_concept_keeps_its_frontmatter_and_body() {
    let bundle = Bundle::load(&fixture("clean")).expect("loads");

    let orders = bundle.concept("tables/orders").expect("orders is present");
    assert_eq!(orders.frontmatter().concept_type(), Some("BigQuery Table"));
    assert!(orders.body().as_str().contains("# Schema"));
}

/// A well-formed bundle reports nothing — the green case the red fixtures are
/// measured against.
#[test]
fn a_clean_bundle_reports_nothing() {
    let bundle = Bundle::load(&fixture("clean")).expect("loads");
    assert!(
        bundle.findings().is_empty(),
        "expected no findings, got: {:?}",
        bundle.findings()
    );
}

/// A `.md` that does not parse as a concept is not added to the bundle, and is
/// reported as CONCEPT-1 located at its file.
#[test]
fn an_unparseable_file_is_not_a_concept_and_is_reported() {
    let bundle = Bundle::load(&fixture("unreadable")).expect("loads");

    assert!(bundle.is_empty(), "the prose-only file is not a concept");
    assert_eq!(bundle.findings().len(), 1);
    assert_eq!(bundle.findings()[0].rule, Rule::NotAConcept);
    assert_eq!(bundle.findings()[0].file, "prose-only.md");
}

/// A concept with no `type` still has identity and is loaded, but is reported as
/// CONCEPT-2 — a defect to fix, not a reason to drop the concept.
#[test]
fn a_typeless_concept_is_loaded_but_reported() {
    let bundle = Bundle::load(&fixture("missing-type")).expect("loads");

    assert_eq!(bundle.len(), 1);
    assert!(bundle.concept("untyped").is_some());
    assert_eq!(bundle.findings().len(), 1);
    assert_eq!(bundle.findings()[0].rule, Rule::MissingType);
    assert_eq!(bundle.findings()[0].file, "untyped.md");
}

/// The clean bundle's body links resolve — both a bundle-absolute and a
/// relative form — to edges, and it still reports nothing.
#[test]
fn resolved_body_links_become_edges() {
    let bundle = Bundle::load(&fixture("clean")).expect("loads");

    let mut edges: Vec<(&str, &str)> = bundle
        .links()
        .iter()
        .map(|e| (e.from.as_str(), e.to.as_str()))
        .collect();
    edges.sort();
    assert_eq!(
        edges,
        [
            ("overview", "tables/customers"),
            ("overview", "tables/orders"),
        ]
    );
    assert!(bundle.findings().is_empty());
}

/// A dangling body link is surfaced as a BUNDLE-2 report and does not become an
/// edge — but it is a report, not a defect, so it must never fail a run (§6).
#[test]
fn a_dangling_link_is_reported_as_a_report_not_a_defect() {
    let bundle = Bundle::load(&fixture("dangling")).expect("loads");

    assert!(bundle.links().is_empty());
    assert_eq!(bundle.findings().len(), 1);
    assert_eq!(bundle.findings()[0].rule, Rule::DanglingLink);
    assert_eq!(bundle.findings()[0].file, "note.md");
    assert!(
        bundle
            .findings()
            .iter()
            .all(|f| f.severity() == Severity::Report),
        "a dangling link must not be a defect"
    );
}

/// A declared `status` outside the draft/stable/deprecated set is reported as
/// CONCEPT-3; the concept still loads — a bad status is a defect, not a reason
/// to drop it.
#[test]
fn an_invalid_status_is_reported() {
    let bundle = Bundle::load(&fixture("bad-status")).expect("loads");

    assert_eq!(bundle.len(), 1);
    assert_eq!(bundle.findings().len(), 1);
    assert_eq!(bundle.findings()[0].rule, Rule::InvalidStatus);
    assert_eq!(bundle.findings()[0].file, "thing.md");
}

/// A declared `generated` block with no `by` is reported as CONCEPT-4 (§5.2).
#[test]
fn generated_without_by_is_reported() {
    let bundle = Bundle::load(&fixture("bad-generated")).expect("loads");

    assert_eq!(bundle.findings().len(), 1);
    assert_eq!(bundle.findings()[0].rule, Rule::MissingGeneratedBy);
    assert_eq!(bundle.findings()[0].file, "thing.md");
}

/// A `by` that is a bare token, not `producer/version` or `scheme:id`, is
/// reported as CONCEPT-5 (§7).
#[test]
fn a_malformed_actor_is_reported() {
    let bundle = Bundle::load(&fixture("bad-actor")).expect("loads");

    assert_eq!(bundle.findings().len(), 1);
    assert_eq!(bundle.findings()[0].rule, Rule::MalformedActor);
    assert_eq!(bundle.findings()[0].file, "thing.md");
}

/// A `sources` entry with no `resource` is reported as CONCEPT-6 (§5.1).
#[test]
fn a_source_without_resource_is_reported() {
    let bundle = Bundle::load(&fixture("bad-sources")).expect("loads");

    assert_eq!(bundle.findings().len(), 1);
    assert_eq!(bundle.findings()[0].rule, Rule::MissingSourceResource);
    assert_eq!(bundle.findings()[0].file, "thing.md");
}

/// A well-formed Attested Computation — `runtime` present, exactly one
/// computation source — reports nothing.
#[test]
fn a_valid_attested_computation_reports_nothing() {
    let bundle = Bundle::load(&fixture("attested-computation")).expect("loads");
    assert!(
        bundle.findings().is_empty(),
        "expected no findings, got: {:?}",
        bundle.findings()
    );
}

/// An Attested Computation with no `runtime` is CONCEPT-7 (§10.2).
#[test]
fn an_attested_computation_without_runtime_is_reported() {
    let bundle = Bundle::load(&fixture("ac-no-runtime")).expect("loads");

    assert_eq!(bundle.findings().len(), 1);
    assert_eq!(bundle.findings()[0].rule, Rule::MissingRuntime);
    assert_eq!(bundle.findings()[0].file, "thing.md");
}

/// A computation given in both a `computation:` path and a `# Computation` body
/// block is CONCEPT-8 (§10.3).
#[test]
fn an_attested_computation_with_both_sources_is_reported() {
    let bundle = Bundle::load(&fixture("ac-both")).expect("loads");

    assert_eq!(bundle.findings().len(), 1);
    assert_eq!(bundle.findings()[0].rule, Rule::InvalidComputationSource);
}

/// A computation given in neither place is also CONCEPT-8 (§10.3).
#[test]
fn an_attested_computation_with_neither_source_is_reported() {
    let bundle = Bundle::load(&fixture("ac-neither")).expect("loads");

    assert_eq!(bundle.findings().len(), 1);
    assert_eq!(bundle.findings()[0].rule, Rule::InvalidComputationSource);
}

/// Path-valued fields that resolve — a `references/` file, a `/`-rooted concept
/// path — report nothing, and a URL and a scope-descriptor source are left
/// alone (not mistaken for dangling paths).
#[test]
fn resolving_path_valued_fields_report_nothing() {
    let bundle = Bundle::load(&fixture("paths")).expect("loads");
    assert!(
        bundle.findings().is_empty(),
        "expected no findings, got: {:?}",
        bundle.findings()
    );
}

/// A path-valued field naming a file that is not in the bundle is a BUNDLE-3
/// report — tolerated, never a defect (§6/§11).
#[test]
fn a_dangling_path_valued_field_is_a_report() {
    let bundle = Bundle::load(&fixture("dangling-path")).expect("loads");

    assert_eq!(bundle.findings().len(), 1);
    assert_eq!(bundle.findings()[0].rule, Rule::DanglingPath);
    assert_eq!(bundle.findings()[0].file, "revenue.md");
    assert!(bundle
        .findings()
        .iter()
        .all(|f| f.severity() == Severity::Report));
}

/// A `sources[].resource` that resolves to a concept is a derivation edge; the
/// ancestor walk follows the chain, and a URL source is a leaf (no edge).
#[test]
fn a_derivation_chain_builds_edges_and_propagates() {
    let bundle = Bundle::load(&fixture("derivation")).expect("loads");

    let edges: Vec<(&str, &str)> = bundle
        .derivations()
        .iter()
        .map(|d| (d.from.as_str(), d.to.as_str()))
        .collect();
    assert_eq!(edges, [("metric", "revenue"), ("revenue", "policy")]);

    assert_eq!(bundle.derivation_ancestors("metric"), ["policy", "revenue"]);
    assert_eq!(bundle.derivation_ancestors("policy"), Vec::<&str>::new());
    assert!(bundle.findings().is_empty());
}

/// Two concepts that derive from each other form a cycle — surfaced as
/// BUNDLE-4, a report (§11 is silent on acyclicity, so it does not fail a run).
#[test]
fn a_derivation_cycle_is_reported() {
    let bundle = Bundle::load(&fixture("derivation-cycle")).expect("loads");

    assert_eq!(bundle.findings().len(), 1);
    let cycle = &bundle.findings()[0];
    assert_eq!(cycle.rule, Rule::DerivationCycle);
    assert_eq!(cycle.severity(), Severity::Report);
    assert_eq!(cycle.file, "a.md");
    assert!(
        cycle.detail.contains("a → b → a"),
        "detail: {}",
        cycle.detail
    );
}

/// A valid index bundle — a root `index.md` with `okf_version` and entries that
/// resolve (a concept and a subdirectory), plus a nested index — reports
/// nothing.
#[test]
fn a_valid_index_reports_nothing() {
    let bundle = Bundle::load(&fixture("index-valid")).expect("loads");
    assert!(
        bundle.findings().is_empty(),
        "expected no findings, got: {:?}",
        bundle.findings()
    );
}

/// A root `index.md` carrying a key other than `okf_version` is INDEX-1, a
/// defect (§8/§11); the entry still resolves, so it is the only finding.
#[test]
fn an_index_with_illegal_frontmatter_is_a_defect() {
    let bundle = Bundle::load(&fixture("index-bad-frontmatter")).expect("loads");

    assert_eq!(bundle.findings().len(), 1);
    assert_eq!(bundle.findings()[0].rule, Rule::IndexFrontmatter);
    assert_eq!(bundle.findings()[0].severity(), Severity::Defect);
    assert_eq!(bundle.findings()[0].file, "index.md");
}

/// An index entry that resolves to nothing is INDEX-2, a report (§6 tolerates a
/// broken link).
#[test]
fn a_dangling_index_entry_is_a_report() {
    let bundle = Bundle::load(&fixture("index-dangling-entry")).expect("loads");

    assert_eq!(bundle.findings().len(), 1);
    assert_eq!(bundle.findings()[0].rule, Rule::DanglingIndexEntry);
    assert_eq!(bundle.findings()[0].severity(), Severity::Report);
    assert_eq!(bundle.findings()[0].file, "index.md");
}

/// A well-ordered, ISO-dated log whose entry resolves reports nothing.
#[test]
fn a_valid_log_reports_nothing() {
    let bundle = Bundle::load(&fixture("log-valid")).expect("loads");
    assert!(
        bundle.findings().is_empty(),
        "expected no findings, got: {:?}",
        bundle.findings()
    );
}

/// A non-ISO log date heading is LOG-1, a defect (§9 makes the date a MUST).
#[test]
fn a_non_iso_log_date_is_a_defect() {
    let bundle = Bundle::load(&fixture("log-bad-date")).expect("loads");

    assert_eq!(bundle.findings().len(), 1);
    assert_eq!(bundle.findings()[0].rule, Rule::NonIsoLogDate);
    assert_eq!(bundle.findings()[0].severity(), Severity::Defect);
    assert_eq!(bundle.findings()[0].file, "log.md");
}

/// Headings not in newest-first order are LOG-2, a report (§9 states the order
/// but does not mark it a MUST).
#[test]
fn an_out_of_order_log_is_a_report() {
    let bundle = Bundle::load(&fixture("log-out-of-order")).expect("loads");

    assert_eq!(bundle.findings().len(), 1);
    assert_eq!(bundle.findings()[0].rule, Rule::LogOutOfOrder);
    assert_eq!(bundle.findings()[0].severity(), Severity::Report);
}

/// A log entry that resolves to nothing is LOG-3, a report (§6 tolerates it).
#[test]
fn a_dangling_log_entry_is_a_report() {
    let bundle = Bundle::load(&fixture("log-dangling-entry")).expect("loads");

    assert_eq!(bundle.findings().len(), 1);
    assert_eq!(bundle.findings()[0].rule, Rule::DanglingLogEntry);
    assert_eq!(bundle.findings()[0].severity(), Severity::Report);
    assert_eq!(bundle.findings()[0].file, "log.md");
}

/// A reference-style body link (`[text][ref]` + `[ref]: /b.md`) resolves to a
/// concept and becomes an edge, end to end — no dangling report.
#[test]
fn a_reference_style_link_becomes_an_edge() {
    let bundle = Bundle::load(&fixture("reference-links")).expect("loads");

    let edges: Vec<(&str, &str)> = bundle
        .links()
        .iter()
        .map(|e| (e.from.as_str(), e.to.as_str()))
        .collect();
    assert_eq!(edges, [("a", "b")]);
    assert!(bundle.findings().is_empty());
}

/// A concept's parent is the nearest path ancestor that is itself a concept;
/// a directory-only scope (no concept file) contributes none.
#[test]
fn parent_is_the_nearest_concept_ancestor() {
    let bundle = Bundle::load(&fixture("hierarchy")).expect("loads");

    assert_eq!(bundle.parent("datasets"), None);
    assert_eq!(bundle.parent("datasets/sales"), Some("datasets"));
    assert_eq!(
        bundle.parent("datasets/sales/detail"),
        Some("datasets/sales")
    );
    assert_eq!(bundle.parent("orphan/deep"), None);
}

/// Children invert the parent relation, so a grandchild attaches to its nearest
/// concept ancestor rather than to every ancestor above it.
#[test]
fn children_attach_to_their_nearest_concept_ancestor() {
    let bundle = Bundle::load(&fixture("hierarchy")).expect("loads");

    assert_eq!(bundle.children("datasets"), vec!["datasets/sales"]);
    assert_eq!(
        bundle.children("datasets/sales"),
        vec!["datasets/sales/detail"]
    );
    assert!(bundle.children("orphan/deep").is_empty());
}
