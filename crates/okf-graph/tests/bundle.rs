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
