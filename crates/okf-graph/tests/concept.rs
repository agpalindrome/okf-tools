//! Acceptance tests for the concept-document model (issue #35, SPEC v0.2 §4).
//!
//! Green: the spec's own worked examples, every field §4.1 names reading back.
//! Red: the four shapes that are not concept documents, plus the near misses
//! that *look* readable — a bare-string frontmatter block, a tag list with a
//! non-string in it, a `---` in the prose that is a horizontal rule.

use okf_graph::{Concept, ConceptError, Status};

/// SPEC §4.3, abridged: a concept bound to a resource, stating every field
/// §4.1 names — plus a §5 trust family this crate carries without reading.
const RESOURCE_CONCEPT: &str = "\
---
type: BigQuery Table
title: Customer Orders
description: One row per completed customer order across all channels.
resource: https://example.com/bigquery?p=acme&d=sales&t=orders
tags: [sales, orders, revenue]
generated: { by: reference_agent/gemini-2.5-pro, at: 2026-05-28T14:30:00Z }
---

# Schema

| Column     | Type   | Description                       |
|------------|--------|-----------------------------------|
| `order_id` | STRING | Globally unique order identifier. |
";

/// SPEC §4.4, abridged: a concept bound to no resource, its body linking to
/// another.
const PLAYBOOK_CONCEPT: &str = "\
---
type: Playbook
title: \"Incident response: data freshness alert\"
description: Steps to triage a freshness alert on the orders pipeline.
tags: [oncall, incident]
generated: { by: human:ahormati, at: 2026-04-12T09:00:00Z }
---

# Trigger

A freshness alert fires when `orders` lags behind its SLA. See the
[orders table](/tables/orders.md).
";

#[test]
fn reads_every_field_the_spec_names() {
    let concept = Concept::parse(RESOURCE_CONCEPT).expect("a §4.3 concept parses");
    let front = concept.frontmatter();

    assert_eq!(front.concept_type(), Some("BigQuery Table"));
    assert_eq!(front.title(), Some("Customer Orders"));
    assert_eq!(
        front.description(),
        Some("One row per completed customer order across all channels.")
    );
    assert_eq!(
        front.resource(),
        Some("https://example.com/bigquery?p=acme&d=sales&t=orders")
    );
    assert_eq!(front.tags(), Some(vec!["sales", "orders", "revenue"]));
}

/// v0.2 retired `timestamp` for `generated.at`, and lets a consumer fall back
/// to it on a v0.1 document — so the accessor stays, as a fallback.
#[test]
fn a_v0_1_timestamp_still_reads() {
    let concept = Concept::parse("---\ntype: Metric\ntimestamp: 2026-05-28T14:30:00Z\n---\n")
        .expect("parses");

    assert_eq!(
        concept.frontmatter().timestamp(),
        Some("2026-05-28T14:30:00Z")
    );
}

/// The §5 provenance, trust, and lifecycle families are not read here, and
/// survive whole in the block — unread is not the same as dropped.
#[test]
fn an_unread_family_survives_in_the_block() {
    let concept = Concept::parse(RESOURCE_CONCEPT).expect("a §4.3 concept parses");

    assert!(concept
        .frontmatter()
        .source()
        .contains("generated: { by: reference_agent/gemini-2.5-pro, at: 2026-05-28T14:30:00Z }"));
}

/// A field the document does not state is absent — not an error, not `""`.
#[test]
fn an_absent_field_is_absent() {
    let concept = Concept::parse(PLAYBOOK_CONCEPT).expect("a §4.4 concept parses");

    assert_eq!(concept.frontmatter().resource(), None);
    assert_eq!(concept.frontmatter().concept_type(), Some("Playbook"));
}

/// The body is everything after the closing fence, verbatim — the blank line
/// that follows it and the trailing newline included.
#[test]
fn the_body_is_everything_after_the_closing_fence() {
    let concept = Concept::parse(PLAYBOOK_CONCEPT).expect("a §4.4 concept parses");

    assert_eq!(
        concept.body().as_str(),
        "\n# Trigger\n\nA freshness alert fires when `orders` lags behind its SLA. See the\n\
         [orders table](/tables/orders.md).\n"
    );
}

/// A document that ends with its frontmatter has nothing to say, not a defect.
#[test]
fn a_document_with_no_body_has_an_empty_body() {
    let concept = Concept::parse("---\ntype: Reference\n---\n").expect("parses");

    assert_eq!(concept.body().as_str(), "");
}

/// Only the first line can open a block, so a `---` in the prose is a
/// horizontal rule and stays in the body where it was written.
#[test]
fn a_horizontal_rule_in_the_body_is_body() {
    let concept =
        Concept::parse("---\ntype: Reference\n---\nabove\n\n---\n\nbelow\n").expect("parses");

    assert_eq!(concept.frontmatter().concept_type(), Some("Reference"));
    assert_eq!(concept.body().as_str(), "above\n\n---\n\nbelow\n");
}

/// The accessors cover the fields §4.1 names; the keys producers add survive
/// in the block's own text, which is what §4.1 asks of a consumer.
#[test]
fn extension_keys_are_preserved() {
    let source = "---\ntype: Metric\nowner: analytics-team\nsubjects:\n  - revenue\n---\nbody\n";
    let concept = Concept::parse(source).expect("parses");

    assert_eq!(
        concept.frontmatter().source(),
        "type: Metric\nowner: analytics-team\nsubjects:\n  - revenue\n"
    );
}

/// A document without the required `type` still parses: a consumer that cannot
/// construct a non-conformant document cannot report anything located about it.
#[test]
fn a_document_missing_the_required_type_still_parses() {
    let concept = Concept::parse("---\ntitle: Untyped\n---\nbody\n").expect("parses");

    assert_eq!(concept.frontmatter().concept_type(), None);
    assert_eq!(concept.frontmatter().title(), Some("Untyped"));
}

/// An empty block declares nothing — a conformance failure, not a parse one.
#[test]
fn an_empty_frontmatter_block_declares_nothing() {
    let concept = Concept::parse("---\n---\nbody\n").expect("parses");

    assert_eq!(concept.frontmatter().concept_type(), None);
    assert_eq!(concept.frontmatter().source(), "");
    assert_eq!(concept.body().as_str(), "body\n");
}

/// Red: a plain markdown file is not a concept document.
#[test]
fn a_file_with_no_frontmatter_is_rejected() {
    assert_eq!(
        Concept::parse("# Just prose\n\nNo metadata here.\n"),
        Err(ConceptError::MissingFrontmatter)
    );
    assert_eq!(Concept::parse(""), Err(ConceptError::MissingFrontmatter));
}

/// Red: an unclosed fence. Where the prose begins is unknowable, and reading
/// the rest of the file as frontmatter would swallow the body.
#[test]
fn an_unclosed_fence_is_rejected() {
    assert_eq!(
        Concept::parse("---\ntype: Reference\n\n# Schema\n"),
        Err(ConceptError::UnterminatedFrontmatter)
    );
}

/// Red: frontmatter that is not parseable YAML.
#[test]
fn unparseable_frontmatter_is_rejected() {
    let err = Concept::parse("---\ntype: [unclosed\n---\nbody\n").expect_err("does not parse");

    assert!(
        matches!(err, ConceptError::MalformedFrontmatter(_)),
        "expected malformed frontmatter, got {err:?}"
    );
}

/// Red, and the near miss: a block that is readable YAML yet declares no
/// fields. Reading it as "every field absent" would report a missing `type` on
/// a file that never had a metadata block.
#[test]
fn frontmatter_that_is_not_a_mapping_is_rejected() {
    assert_eq!(
        Concept::parse("---\njust a string\n---\nbody\n"),
        Err(ConceptError::FrontmatterNotAMapping)
    );
    assert_eq!(
        Concept::parse("---\n- one\n- two\n---\nbody\n"),
        Err(ConceptError::FrontmatterNotAMapping)
    );
}

/// Red, and the near miss that costs a tag: a list holding a non-string reads
/// as absent, not as the strings beside it. A dropped tag is one nothing looks
/// for again.
#[test]
fn a_tag_list_with_a_non_string_reads_as_absent() {
    let concept =
        Concept::parse("---\ntype: Metric\ntags: [sales, {name: orders}]\n---\n").expect("parses");

    assert_eq!(concept.frontmatter().tags(), None);
}

/// Any field of the wrong shape reads as absent, by the same rule. Telling the
/// two apart is a conformance check's job, over frontmatter kept whole here.
#[test]
fn a_field_of_the_wrong_shape_reads_as_absent() {
    let concept = Concept::parse("---\ntype: 42\ntitle: Answers\n---\n").expect("parses");

    assert_eq!(concept.frontmatter().concept_type(), None);
    assert_eq!(concept.frontmatter().title(), Some("Answers"));
}

/// The lifecycle families (§5.4/§5.5) read back: `status` as an enum, and
/// `stale_after` as the raw date string.
#[test]
fn reads_lifecycle_status_and_stale_after() {
    let concept =
        Concept::parse("---\ntype: Reference\nstatus: deprecated\nstale_after: 2026-09-23\n---\n")
            .expect("parses");

    assert_eq!(concept.frontmatter().status(), Some(Status::Deprecated));
    assert_eq!(concept.frontmatter().stale_after(), Some("2026-09-23"));
}

/// An unrecognised `status` reads as `None`, by the same shape rule as `tags`;
/// telling that from absent (both `None`) is a conformance check's job.
#[test]
fn an_unrecognised_status_reads_as_none() {
    let concept = Concept::parse("---\ntype: Reference\nstatus: archived\n---\n").expect("parses");
    assert_eq!(concept.frontmatter().status(), None);
}

/// An absent lifecycle family is `None` — not an error, and not defaulted here.
#[test]
fn absent_lifecycle_fields_are_none() {
    let concept = Concept::parse("---\ntype: Reference\n---\n").expect("parses");
    assert_eq!(concept.frontmatter().status(), None);
    assert_eq!(concept.frontmatter().stale_after(), None);
}

/// A CRLF file splits like any other: the fences tolerate the carriage return.
#[test]
fn crlf_line_endings_split_the_same_way() {
    let concept = Concept::parse("---\r\ntype: Reference\r\n---\r\nbody\r\n").expect("parses");

    assert_eq!(concept.frontmatter().concept_type(), Some("Reference"));
    assert_eq!(concept.body().as_str(), "body\r\n");
}
