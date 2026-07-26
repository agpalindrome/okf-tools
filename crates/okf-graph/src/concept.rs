//! An OKF **concept document** ([SPEC §4][spec]): one markdown file, read as a
//! YAML [`Frontmatter`] block and a markdown [`Body`]. Those two parts are the
//! whole model.
//!
//! No **concept id**: §2 defines it as the file's path within the bundle, which
//! is a fact about the bundle rather than the document. Identity belongs to
//! whatever loads one.
//!
//! **Parse leniently, judge strictly.** §9 requires consumers to tolerate a
//! missing optional field, an unknown `type`, an unknown extra key, a broken
//! link — so parsing fails only where the two parts cannot be told apart, never
//! over what the frontmatter says. A concept with no `type` fails conformance
//! and still parses: a checker that cannot build a defective document cannot
//! report anything located about it.
//!
//! Written against **v0.2**, which the crate follows as it changes. v0.2 leaves
//! §4 and §11 as they were, retires `timestamp` for `generated.at`, and adds
//! the provenance, trust, and lifecycle families (§5) this crate does not read
//! yet.
//!
//! [spec]: https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md#4-concept-documents

use std::fmt;

use serde_yaml::{Mapping, Value};

/// One OKF concept document: its frontmatter and its body (§4).
#[derive(Debug, Clone, PartialEq)]
pub struct Concept {
    frontmatter: Frontmatter,
    body: Body,
}

impl Concept {
    /// Read a concept document from one markdown file's text: a `---`-fenced
    /// YAML block, then the body verbatim. What can fail is shape, never
    /// content — see [`ConceptError`].
    pub fn parse(source: &str) -> Result<Concept, ConceptError> {
        let (front, body) = split(source)?;
        Ok(Concept {
            frontmatter: Frontmatter::parse(front)?,
            body: Body(body.to_string()),
        })
    }

    /// The document's frontmatter.
    pub fn frontmatter(&self) -> &Frontmatter {
        &self.frontmatter
    }

    /// The document's body.
    pub fn body(&self) -> &Body {
        &self.body
    }
}

/// The YAML metadata block at the top of a concept document (§4.1).
///
/// Every accessor answers `Some` only when the key is present *and* holds the
/// shape §4.1 describes. That conflates "absent" with "present but the wrong
/// shape" deliberately: neither is this type's to judge, and the block is kept
/// whole so a conformance check can tell them apart.
#[derive(Debug, Clone, PartialEq)]
pub struct Frontmatter {
    source: String,
    fields: Mapping,
}

impl Frontmatter {
    /// Parse the YAML text between the `---` fences.
    fn parse(source: &str) -> Result<Frontmatter, ConceptError> {
        let value = serde_yaml::from_str::<Value>(source)
            .map_err(|e| ConceptError::MalformedFrontmatter(e.to_string()))?;
        let fields = match value {
            Value::Mapping(fields) => fields,
            // An empty block parses as null and declares nothing — which is an
            // empty mapping. Its missing `type` is a finding, not a parse error.
            Value::Null => Mapping::new(),
            _ => return Err(ConceptError::FrontmatterNotAMapping),
        };
        Ok(Frontmatter {
            source: source.to_string(),
            fields,
        })
    }

    /// `type` — the one required field, naming the kind of concept. Spelled out
    /// because `type` is a keyword. `None` is a §9 conformance failure to
    /// report, not a reason to refuse the document.
    pub fn concept_type(&self) -> Option<&str> {
        self.string("type")
    }

    /// `title` — display name; a consumer may derive one from the filename.
    pub fn title(&self) -> Option<&str> {
        self.string("title")
    }

    /// `description` — a one-sentence summary.
    pub fn description(&self) -> Option<&str> {
        self.string("description")
    }

    /// `resource` — URI of the asset described; absent for abstract concepts.
    pub fn resource(&self) -> Option<&str> {
        self.string("resource")
    }

    /// `timestamp` — a v0.1 field, superseded by `generated.at` (§13.1) and
    /// read here as the fallback v0.2 allows for older documents. Kept as
    /// written: whether it parses as ISO 8601 is a question about the string,
    /// not about the document.
    pub fn timestamp(&self) -> Option<&str> {
        self.string("timestamp")
    }

    /// `tags` — the categorization strings, or `None` if absent or not a list
    /// of strings. A list holding a non-string reads as `None` rather than the
    /// strings beside it: a silently dropped tag is one nothing looks for again.
    pub fn tags(&self) -> Option<Vec<&str>> {
        match self.fields.get("tags") {
            Some(Value::Sequence(items)) => items
                .iter()
                .map(|item| match item {
                    Value::String(s) => Some(s.as_str()),
                    _ => None,
                })
                .collect(),
            _ => None,
        }
    }

    /// `status` — the lifecycle stage (§5.4), read only when it is one of the
    /// three declared values. `None` covers both absent and an unrecognised
    /// value; absent means the spec's default `stable`, but applying that
    /// default is a consumer's call, not this reader's — okf-graph reports the
    /// declared shape and derives nothing.
    pub fn status(&self) -> Option<Status> {
        match self.string("status")? {
            "draft" => Some(Status::Draft),
            "stable" => Some(Status::Stable),
            "deprecated" => Some(Status::Deprecated),
            _ => None,
        }
    }

    /// `stale_after` — the absolute `YYYY-MM-DD` date on or after which the
    /// concept is stale (§5.5), kept as written. Whether it parses as a date,
    /// and whether today is past it, are questions for a consumer, not this
    /// reader.
    pub fn stale_after(&self) -> Option<&str> {
        self.string("stale_after")
    }

    /// `generated` — how the current content was produced (§5.2): an actor
    /// `by` (required) and an ISO-8601 `at`. `None` when `generated` is absent
    /// or is not a mapping; a mapping with no `by` still reads (as
    /// `Generated { by: None, .. }`), so a check can tell it from absent.
    pub fn generated(&self) -> Option<Generated> {
        let value = self.fields.get("generated")?;
        value.is_mapping().then(|| Generated {
            by: str_at(value, "by"),
            at: str_at(value, "at"),
        })
    }

    /// `verified` — the verification events (§5.2), each an actor `by` and an
    /// ISO-8601 `at`. A **bare `{ by, at }` mapping counts as one** event, not
    /// zero (§5.2 MUST); a list reads each entry; absent reads as empty.
    pub fn verified(&self) -> Vec<Verification> {
        match self.fields.get("verified") {
            Some(Value::Sequence(items)) => items.iter().filter_map(verification).collect(),
            Some(value @ Value::Mapping(_)) => verification(value).into_iter().collect(),
            _ => Vec::new(),
        }
    }

    /// `sources` — the materials a concept derives from (§5.1). Each entry's
    /// `resource` is required, but a missing one still reads (as
    /// `Source { resource: None, .. }`) so a check can locate it; non-mapping
    /// entries are dropped, and an absent or non-list `sources` reads as empty.
    pub fn sources(&self) -> Vec<Source> {
        match self.fields.get("sources") {
            Some(Value::Sequence(items)) => items.iter().filter_map(source_entry).collect(),
            _ => Vec::new(),
        }
    }

    /// `usage_window` — the shared `{ from, to }` range that frames every
    /// source's `usage_count` (§5.1). A single source may carry its own; this
    /// reads the bundle-wide sibling.
    pub fn usage_window(&self) -> Option<UsageWindow> {
        self.fields.get("usage_window").and_then(usage_window)
    }

    /// `runtime` — how an Attested Computation is run (§10.2), which fixes what
    /// its `parameters` mean. Required for that type, but read like any other
    /// string: `None` is absent or not a string, and a check reports it.
    pub fn runtime(&self) -> Option<&str> {
        self.string("runtime")
    }

    /// `computation` — a path (§6.2) to a file holding an Attested
    /// Computation's computation, used instead of the inline body block (§10.3).
    pub fn computation(&self) -> Option<&str> {
        self.string("computation")
    }

    /// `parameters` — the typed, named holes an Attested Computation exposes
    /// (§10.2), each `{ name, type, required }`. Absent or non-list reads empty;
    /// non-mapping entries are dropped.
    pub fn parameters(&self) -> Vec<Parameter> {
        match self.fields.get("parameters") {
            Some(Value::Sequence(items)) => items.iter().filter_map(parameter).collect(),
            _ => Vec::new(),
        }
    }

    /// `executor` — how an Attested Computation is run (§10.2): a `resource`
    /// naming the run instructions, and the `receipt` fields a run must return.
    pub fn executor(&self) -> Option<Executor> {
        let value = self.fields.get("executor")?;
        value.is_mapping().then(|| Executor {
            resource: str_at(value, "resource"),
            receipt: string_list(value, "receipt"),
        })
    }

    /// `attester` — the deterministic, consumer-side check (§10.2): a `resource`
    /// naming code that takes a receipt and returns a verdict.
    pub fn attester(&self) -> Option<Attester> {
        let value = self.fields.get("attester")?;
        value.is_mapping().then(|| Attester {
            resource: str_at(value, "resource"),
        })
    }

    /// The block exactly as written, fences excluded. §4.1 lets producers add
    /// any keys and *requires* consumers not to reject unknown ones, so
    /// extension keys — and the §5 families — survive here: the payload a
    /// semantic layer reads and this crate does not interpret.
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Whether the block declares `key` at all, of any shape. Lets a check tell
    /// "absent" (fine) from "present but the wrong shape" (a finding) — the
    /// distinction the `Some`-only-on-shape readers deliberately drop.
    pub(crate) fn declares(&self, key: &str) -> bool {
        self.fields.contains_key(key)
    }

    /// The string at `key`, if the block holds a string there.
    fn string(&self, key: &str) -> Option<&str> {
        match self.fields.get(key) {
            Some(Value::String(s)) => Some(s.as_str()),
            _ => None,
        }
    }
}

/// A concept's lifecycle `status` (§5.4). Absent defaults to `Stable`, but that
/// default is a consumer's to apply — see [`Frontmatter::status`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// Not yet reviewed; possibly incomplete.
    Draft,
    /// Ready for consumption (the default when `status` is absent).
    Stable,
    /// Kept for links and history; no longer current.
    Deprecated,
}

/// The `generated` trust family (§5.2): who produced the current content and
/// when. Both are read as written; `by` should be an actor (§7) and `at` an
/// ISO-8601 datetime, but whether they are is a check's or a consumer's call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Generated {
    /// The actor that produced the content (§7). Required by §5.2.
    pub by: Option<String>,
    /// The ISO-8601 datetime of the last meaningful change.
    pub at: Option<String>,
}

/// One `verified` event (§5.2): who confirmed the content, and when.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verification {
    /// The actor that confirmed the content (§7).
    pub by: Option<String>,
    /// The ISO-8601 datetime of the confirmation.
    pub at: Option<String>,
}

/// One `sources` entry (§5.1): a material a concept derives from, and the
/// per-source credibility signals. `resource` is required by the spec, but is
/// `Option` here so a missing one can be read and then reported.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Source {
    /// A stable key used to attribute individual claims.
    pub id: Option<String>,
    /// The artifact or scope the concept derives from (required by §5.1).
    pub resource: Option<String>,
    /// A human-readable label.
    pub title: Option<String>,
    /// Who or what produced the source, in the actor convention (§7).
    pub author: Option<String>,
    /// How often `resource` was exercised over the `usage_window`.
    pub usage_count: Option<i64>,
    /// When the source itself last changed (`YYYY-MM-DD`).
    pub last_modified: Option<String>,
    /// A per-source `{ from, to }` range overriding the shared one.
    pub usage_window: Option<UsageWindow>,
}

/// The `{ from, to }` date range that frames a `usage_count` (§5.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsageWindow {
    /// Start of the window (`YYYY-MM-DD`).
    pub from: Option<String>,
    /// End of the window (`YYYY-MM-DD`).
    pub to: Option<String>,
}

/// Read one `{ by, at }` event from a value, if it is a mapping.
fn verification(value: &Value) -> Option<Verification> {
    value.is_mapping().then(|| Verification {
        by: str_at(value, "by"),
        at: str_at(value, "at"),
    })
}

/// Read one `sources` entry from a value, if it is a mapping.
fn source_entry(value: &Value) -> Option<Source> {
    value.is_mapping().then(|| Source {
        id: str_at(value, "id"),
        resource: str_at(value, "resource"),
        title: str_at(value, "title"),
        author: str_at(value, "author"),
        usage_count: value.get("usage_count").and_then(Value::as_i64),
        last_modified: str_at(value, "last_modified"),
        usage_window: value.get("usage_window").and_then(usage_window),
    })
}

/// Read a `{ from, to }` window from a value, if it is a mapping.
fn usage_window(value: &Value) -> Option<UsageWindow> {
    value.is_mapping().then(|| UsageWindow {
        from: str_at(value, "from"),
        to: str_at(value, "to"),
    })
}

/// A typed, named hole an Attested Computation exposes (§10.2). `name`/`type`/
/// `required` are all present in a well-formed entry; each is `Option` so a
/// malformed one still reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parameter {
    /// The parameter's name — the hole an agent fills.
    pub name: Option<String>,
    /// Its type, interpreted per the concept's `runtime`. Named `kind` because
    /// `type` is a Rust keyword.
    pub kind: Option<String>,
    /// Whether a value must be supplied.
    pub required: Option<bool>,
}

/// How an Attested Computation is run (§10.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Executor {
    /// A path (§6.2) to the run instructions or code.
    pub resource: Option<String>,
    /// The fields a run must return, the evidence the attester inspects.
    pub receipt: Vec<String>,
}

/// The deterministic, consumer-side check of an Attested Computation (§10.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attester {
    /// A path (§6.2) to the checker code (no LLM).
    pub resource: Option<String>,
}

/// Read one `parameters` entry from a value, if it is a mapping.
fn parameter(value: &Value) -> Option<Parameter> {
    value.is_mapping().then(|| Parameter {
        name: str_at(value, "name"),
        kind: str_at(value, "type"),
        required: value.get("required").and_then(Value::as_bool),
    })
}

/// The list of strings at `value[key]`, or empty when absent or not a list of
/// strings; a non-string item is dropped.
fn string_list(value: &Value, key: &str) -> Vec<String> {
    match value.get(key) {
        Some(Value::Sequence(items)) => items
            .iter()
            .filter_map(|item| match item {
                Value::String(s) => Some(s.clone()),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// The string at `value[key]`, if `value` is a mapping holding a string there.
fn str_at(value: &Value, key: &str) -> Option<String> {
    match value.get(key) {
        Some(Value::String(s)) => Some(s.clone()),
        _ => None,
    }
}

/// Everything after the frontmatter (§4.2): markdown, carried verbatim and not
/// parsed. Links between concepts live here (§5), so topology will be read from
/// it later.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Body(String);

impl Body {
    /// The body text as written.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Whether the body carries a `# Computation` section (§10.3) — the inline
    /// half of the Attested-Computation computation-XOR.
    ///
    /// Keyed on the heading's presence, at any level and outside fenced code,
    /// **not** on the code block's style: §10.3 says "fenced", but §10.2's own
    /// example indents it (`docs/okf-friction.md`), so requiring a fence would
    /// miss the spec's example. Whether the section actually holds a block is a
    /// finer check, deferred (#58).
    pub fn has_computation_section(&self) -> bool {
        let mut in_fence = false;
        for line in self.0.lines() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
                in_fence = !in_fence;
            } else if !in_fence
                && trimmed.starts_with('#')
                && trimmed.trim_start_matches('#').trim() == "Computation"
            {
                return true;
            }
        }
        false
    }
}

/// The ways a markdown file can fail to *be* a concept document — all about
/// shape, none about content. §9 requires tolerating a missing `type`, an
/// unknown key, or a broken link, and a checker cannot report on a document it
/// refused to parse.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConceptError {
    /// The file does not open with a `---` fence.
    MissingFrontmatter,
    /// The opening fence is never closed, so where the body begins is unknown.
    UnterminatedFrontmatter,
    /// The frontmatter is not parseable YAML; carries the parser's message.
    MalformedFrontmatter(String),
    /// The frontmatter parses as a scalar or a list, declaring no fields at all
    /// — not the same as a block whose fields are absent.
    FrontmatterNotAMapping,
}

impl fmt::Display for ConceptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConceptError::MissingFrontmatter => {
                write!(
                    f,
                    "no frontmatter: the file does not open with a `---` fence"
                )
            }
            ConceptError::UnterminatedFrontmatter => {
                write!(
                    f,
                    "unterminated frontmatter: the opening `---` fence is never closed"
                )
            }
            ConceptError::MalformedFrontmatter(e) => {
                write!(f, "frontmatter is not parseable YAML: {e}")
            }
            ConceptError::FrontmatterNotAMapping => {
                write!(f, "frontmatter is not a mapping, so it declares no fields")
            }
        }
    }
}

impl std::error::Error for ConceptError {}

/// Split a file into frontmatter text and body. Only the first line can open a
/// block, so a `---` in the prose is a horizontal rule; the closing fence is the
/// first `---` after it. Trimming both makes a CRLF file split the same way.
fn split(source: &str) -> Result<(&str, &str), ConceptError> {
    let mut lines = source.split_inclusive('\n');
    let opening = lines.next().ok_or(ConceptError::MissingFrontmatter)?;
    if opening.trim_end() != "---" {
        return Err(ConceptError::MissingFrontmatter);
    }
    let start = opening.len();
    let mut offset = start;
    for line in source[start..].split_inclusive('\n') {
        if line.trim_end() == "---" {
            return Ok((&source[start..offset], &source[offset + line.len()..]));
        }
        offset += line.len();
    }
    Err(ConceptError::UnterminatedFrontmatter)
}
