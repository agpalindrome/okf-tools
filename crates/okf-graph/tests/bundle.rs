//! Acceptance tests for `Bundle::load` (issue #40): identity and reserved-file
//! exclusion over a clean multi-directory bundle.

use std::path::PathBuf;

use okf_graph::{Bundle, Rule};

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
