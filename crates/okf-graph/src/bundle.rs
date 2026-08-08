//! A whole OKF Knowledge Bundle loaded into memory: the concepts a directory
//! tree yields, keyed by Concept ID, and the findings the load produced.
//!
//! The walk reuses the shape of `deon`'s `okf.rs::collect` — recurse a
//! directory for `*.md`, read each file — but this crate owns identity: a
//! Concept ID is the file's bundle-relative path with `.md` removed (SPEC §2),
//! and the reserved `index.md` / `log.md` are excluded from the concept set
//! (§3.1); their own structure is validated elsewhere.

use std::collections::btree_map::{BTreeMap, Entry};
use std::collections::BTreeSet;
use std::path::Path;

use crate::concept::split_frontmatter;
use crate::index;
use crate::links::{links_in, Link, LinkKind};
use crate::log;
use crate::paths::{classify_path, resolve_path, PathKind};
use crate::provenance::{self, Derivation};
use crate::{Concept, Date, Finding, Level, Policy, Rule, Timestamp, UsageWindow};

/// A resolved body-link edge: the linking concept points at another concept in
/// the same bundle (SPEC §6). A link that resolves to no concept is a dangling
/// `BUNDLE-2` report instead, and never an edge.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct BodyLink {
    /// Concept ID of the concept whose body carries the link.
    pub from: String,
    /// Concept ID the link resolves to.
    pub to: String,
}

/// An OKF Knowledge Bundle: its concepts by Concept ID, the resolved body-link
/// edges, and the findings the load produced.
#[derive(Debug, Clone, Default)]
pub struct Bundle {
    concepts: BTreeMap<String, Concept>,
    files: BTreeSet<String>,
    reserved: Vec<(String, String)>,
    links: Vec<BodyLink>,
    derivations: Vec<Derivation>,
    findings: Vec<Finding>,
}

impl Bundle {
    /// Load a bundle from a directory tree.
    ///
    /// Every `*.md` outside the reserved names becomes a concept keyed by its
    /// Concept ID. IO errors (an unreadable directory or file) propagate; a
    /// file that is not a well-formed concept does not — it becomes a finding,
    /// so one malformed document never blocks the rest of the bundle.
    ///
    /// Symlink entries below `root` are not documents (see `collect`), but
    /// `root` itself is resolved: the caller named that directory, whatever it
    /// is on disk.
    pub fn load(root: &Path) -> std::io::Result<Bundle> {
        if !root.is_dir() {
            return Err(std::io::Error::other(format!(
                "bundle path is not a directory: {}",
                root.display()
            )));
        }
        let mut bundle = Bundle::default();
        collect(root, root, &mut bundle)?;
        bundle.resolve_links();
        bundle.resolve_paths();
        bundle.resolve_provenance();
        bundle.resolve_reserved();
        Ok(bundle)
    }

    /// The concept with this Concept ID, if the bundle has one.
    pub fn concept(&self, id: &str) -> Option<&Concept> {
        self.concepts.get(id)
    }

    /// Every concept, as `(id, concept)` pairs, in Concept ID order.
    pub fn concepts(&self) -> impl Iterator<Item = (&str, &Concept)> {
        self.concepts.iter().map(|(id, c)| (id.as_str(), c))
    }

    /// Every resolved body-link edge, in `(from, to)` Concept ID order of the
    /// linking concept, then document order within a body.
    pub fn links(&self) -> &[BodyLink] {
        &self.links
    }

    /// Every derivation edge (§5.1): a `sources[].resource` that resolves to a
    /// concept, so `from` derives from `to`.
    pub fn derivations(&self) -> &[Derivation] {
        &self.derivations
    }

    /// The concepts `id` transitively derives from — the credibility
    /// propagation walk (§5.1), sorted, deduplicated, and cycle-safe.
    pub fn derivation_ancestors(&self, id: &str) -> Vec<&str> {
        provenance::ancestors(&self.derivations, id)
    }

    /// The parent of a concept in the directory hierarchy (§3): the nearest
    /// path ancestor that is itself a concept. `datasets/sales/detail` →
    /// `datasets/sales` if that is a concept, else `datasets`, else `None` — a
    /// directory with only an `index.md` is a scope, not a concept, so it links
    /// nothing.
    pub fn parent<'a>(&'a self, id: &str) -> Option<&'a str> {
        let mut rest = id;
        while let Some(slash) = rest.rfind('/') {
            let ancestor = &rest[..slash];
            if let Some((key, _)) = self.concepts.get_key_value(ancestor) {
                return Some(key.as_str());
            }
            rest = ancestor;
        }
        None
    }

    /// The concepts whose parent is `id`, in Concept ID order — the inverse of
    /// [`parent`](Self::parent), so a grandchild attaches to its nearest concept
    /// ancestor, not to every ancestor.
    pub fn children<'a>(&'a self, id: &str) -> Vec<&'a str> {
        self.concepts
            .keys()
            .filter(|child| self.parent(child) == Some(id))
            .map(String::as_str)
            .collect()
    }

    /// Every finding the load produced, at the spec's own severities.
    pub fn findings(&self) -> &[Finding] {
        &self.findings
    }

    /// The findings `policy` does not silence, in the order they were found.
    ///
    /// Filtering rather than skipping the work: every check runs during
    /// [`load`], so a silenced rule costs nothing to have computed, and a
    /// consumer that changes its mind about a level does not reload the bundle.
    ///
    /// [`load`]: Bundle::load
    pub fn findings_at(&self, policy: &Policy) -> Vec<&Finding> {
        self.findings
            .iter()
            .filter(|finding| policy.level(finding.rule) > Level::Allow)
            .collect()
    }

    /// Whether the bundle fails under `policy` — whether any finding reaches
    /// [`Level::Defect`]. This is the exit code's question, and the only one a
    /// caller has to ask to gate on it.
    pub fn fails(&self, policy: &Policy) -> bool {
        self.findings
            .iter()
            .any(|finding| policy.level(finding.rule) == Level::Defect)
    }

    /// Number of concepts (reserved files excluded).
    pub fn len(&self) -> usize {
        self.concepts.len()
    }

    /// Whether the bundle has no concepts.
    pub fn is_empty(&self) -> bool {
        self.concepts.is_empty()
    }

    /// Add a concept under its id, or report a duplicate.
    ///
    /// Distinct files give distinct ids within one tree, so `BUNDLE-1` cannot
    /// fire from a well-formed bundle; the guard is defensive (a merged or
    /// overlaid bundle, or symlink/case-fold aliasing, could collide). The first
    /// file to claim an id keeps it, so resolution stays deterministic.
    fn add_concept(&mut self, id: String, file: String, concept: Concept) {
        match self.concepts.entry(id) {
            Entry::Vacant(slot) => {
                slot.insert(concept);
            }
            Entry::Occupied(slot) => {
                let detail = format!(
                    "Concept ID `{}` is already declared by another file",
                    slot.key()
                );
                self.findings
                    .push(Finding::new(file, Rule::DuplicateId, detail));
            }
        }
    }

    /// Resolve every concept's body links against the loaded concept set: a link
    /// that lands on a concept becomes an edge, one that lands nowhere becomes a
    /// dangling `BUNDLE-2` report. Runs after every concept is known, so a
    /// forward reference resolves regardless of load order.
    fn resolve_links(&mut self) {
        let mut edges = Vec::new();
        let mut findings = Vec::new();
        for (from, concept) in &self.concepts {
            for link in links_in(concept.body().as_str()) {
                let Some(to) = resolve_concept_target(from, &link) else {
                    continue;
                };
                if self.concepts.contains_key(&to) {
                    edges.push(BodyLink {
                        from: from.clone(),
                        to,
                    });
                } else {
                    findings.push(Finding::new(
                        format!("{from}.md"),
                        Rule::DanglingLink,
                        format!(
                            "link to `{}` resolves to no concept in the bundle",
                            link.target
                        ),
                    ));
                }
            }
        }
        self.links = edges;
        self.findings.extend(findings);
    }

    /// Resolve each concept's path-valued fields (§6.2) against the file set: a
    /// bundle-path or relative target that names no file is a dangling
    /// `BUNDLE-3` report. URLs and `sources` scope descriptors are not paths and
    /// are left alone.
    fn resolve_paths(&mut self) {
        let mut findings = Vec::new();
        for (id, concept) in &self.concepts {
            let fm = concept.frontmatter();
            let mut check = |field: &str, value: Option<&str>, allow_scope: bool| {
                if let Some(value) = value {
                    check_path(id, field, value, allow_scope, &self.files, &mut findings);
                }
            };
            check("resource", fm.resource(), false);
            check("computation", fm.computation(), false);
            for (i, source) in fm.sources().iter().enumerate() {
                check(
                    &format!("sources[{i}].resource"),
                    source.resource.as_deref(),
                    true,
                );
            }
            if let Some(executor) = fm.executor() {
                check("executor.resource", executor.resource.as_deref(), false);
            }
            if let Some(attester) = fm.attester() {
                check("attester.resource", attester.resource.as_deref(), false);
            }
        }
        self.findings.extend(findings);
    }

    /// Build the derivation graph (§5.1): a `sources[].resource` that resolves
    /// to a concept is an edge from the deriving concept to its source. A
    /// source that is a URL, a scope descriptor, or a non-concept file is a
    /// leaf, not an edge.
    fn resolve_provenance(&mut self) {
        let mut edges = Vec::new();
        for (from, concept) in &self.concepts {
            for source in concept.frontmatter().sources() {
                if let Some(resource) = source.resource.as_deref() {
                    if let Some(to) = self.derivation_target(from, resource) {
                        edges.push(Derivation {
                            from: from.clone(),
                            to,
                        });
                    }
                }
            }
        }
        for cycle in provenance::cycles(&edges) {
            let loop_path = cycle
                .iter()
                .chain(std::iter::once(&cycle[0]))
                .cloned()
                .collect::<Vec<_>>()
                .join(" → ");
            self.findings.push(Finding::new(
                format!("{}.md", cycle[0]),
                Rule::DerivationCycle,
                format!("derivation cycle: {loop_path}"),
            ));
        }
        self.derivations = edges;
    }

    /// Check each reserved file's own structure (§8/§9), now that the whole
    /// tree is known so an entry can be resolved against it.
    fn resolve_reserved(&mut self) {
        let reserved = std::mem::take(&mut self.reserved);
        let mut findings = Vec::new();
        for (path, text) in &reserved {
            match reserved_name(path) {
                "index.md" => self.check_index(path, text, &mut findings),
                "log.md" => self.check_log(path, text, &mut findings),
                _ => {}
            }
        }
        self.findings.extend(findings);
    }

    /// Check one `index.md` (§8/§12): its frontmatter rule, a declared
    /// `okf_version`, and that each entry link resolves.
    fn check_index(&self, path: &str, text: &str, findings: &mut Vec<Finding>) {
        let (frontmatter, body) = split_frontmatter(text);
        let is_root = !path.contains('/');
        if let Some(frontmatter) = frontmatter {
            let check = index::check_frontmatter(frontmatter, is_root);
            if !check.illegal_keys.is_empty() {
                let keys = check.illegal_keys.join(", ");
                let detail = if is_root {
                    format!("root index.md may carry only `okf_version`; found: {keys}")
                } else {
                    format!("a nested index.md may carry no frontmatter; found: {keys}")
                };
                findings.push(Finding::new(path, Rule::IndexFrontmatter, detail));
            }
            if let Some(version) = check.unknown_version {
                findings.push(Finding::new(
                    path,
                    Rule::UnknownOkfVersion,
                    format!("okf_version `{version}` is not understood; this tool reads 0.1 and 0.2 (SPEC §12)"),
                ));
            }
        }

        let from = strip_md(path);
        for link in links_in(body) {
            let target = match link.kind {
                LinkKind::BundleAbsolute | LinkKind::Relative => resolve_path(&from, &link.target),
                LinkKind::External | LinkKind::Fragment => continue,
            };
            if !self.entry_resolves(&target) {
                findings.push(Finding::new(
                    path,
                    Rule::DanglingIndexEntry,
                    format!(
                        "index entry `{}` resolves to no concept, file, or directory",
                        link.target
                    ),
                ));
            }
        }
    }

    /// Check one `log.md` (§9): each date heading is ISO-8601, they run
    /// newest-first, and each entry link resolves.
    fn check_log(&self, path: &str, text: &str, findings: &mut Vec<Finding>) {
        let (_, body) = split_frontmatter(text);
        let headings = log::check_headings(body);
        for heading in &headings.non_iso {
            findings.push(Finding::new(
                path,
                Rule::NonIsoLogDate,
                format!("log date heading `{heading}` is not ISO-8601 YYYY-MM-DD (SPEC §9)"),
            ));
        }
        if headings.out_of_order {
            findings.push(Finding::new(
                path,
                Rule::LogOutOfOrder,
                "log date headings are not in newest-first order (SPEC §9)".to_string(),
            ));
        }

        let from = strip_md(path);
        for link in links_in(body) {
            let target = match link.kind {
                LinkKind::BundleAbsolute | LinkKind::Relative => resolve_path(&from, &link.target),
                LinkKind::External | LinkKind::Fragment => continue,
            };
            if !self.entry_resolves(&target) {
                findings.push(Finding::new(
                    path,
                    Rule::DanglingLogEntry,
                    format!(
                        "log entry `{}` resolves to no concept, file, or directory",
                        link.target
                    ),
                ));
            }
        }
    }

    /// Whether a resolved entry target names a file in the bundle or a directory
    /// that holds one (an index may list a subdirectory, §8).
    fn entry_resolves(&self, target: &str) -> bool {
        self.files.contains(target)
            || self.files.iter().any(|f| {
                f.strip_prefix(target)
                    .is_some_and(|rest| rest.starts_with('/'))
            })
    }

    /// The Concept ID a `sources[].resource` derives from, or `None` when it is
    /// not a concept: a URL or scope descriptor (both left alone here), or a
    /// path to a non-`.md` file (an external leaf mirrored under `references/`).
    fn derivation_target(&self, from: &str, resource: &str) -> Option<String> {
        match classify_path(resource, true) {
            PathKind::BundlePath | PathKind::Relative => {
                let path = resolve_path(from, resource);
                let id = path.strip_suffix(".md")?;
                self.concepts.contains_key(id).then(|| id.to_string())
            }
            PathKind::Url | PathKind::ScopeDescriptor => None,
        }
    }
}

/// Recurse `path`, adding every non-reserved `*.md` to `out` as a concept.
/// Directory entries are visited in path order so any per-file reporting is
/// reproducible. A symlink entry is recorded but never read or descended into.
fn collect(root: &Path, path: &Path, out: &mut Bundle) -> std::io::Result<()> {
    if path.is_dir() {
        let mut entries: Vec<_> = std::fs::read_dir(path)?
            .collect::<std::io::Result<Vec<_>>>()?
            .into_iter()
            .map(|e| e.path())
            .collect();
        entries.sort();
        for entry in entries {
            // A symlink aliases a document rather than being one. Reading it
            // would give the same file a second Concept ID, and a directory
            // symlink to an ancestor re-walks the whole tree until the OS
            // refuses to resolve any deeper — a corpus multiplied by however
            // many links the platform allows, which is not a bundle property.
            // It is still recorded in `files`, so a path-valued field (§6.2) or
            // an index entry naming one resolves: whether to follow it is the
            // consumer's business, not this checker's.
            if std::fs::symlink_metadata(&entry)?.file_type().is_symlink() {
                out.files.insert(rel_path(root, &entry));
                continue;
            }
            collect(root, &entry, out)?;
        }
    } else {
        // Record every file so a path-valued field (§6.2) can resolve against
        // the whole tree, not only the concepts. Reserved and non-`.md` files
        // are targets too (`references/foo.py`, `/log.md`).
        let file = rel_path(root, path);
        out.files.insert(file.clone());
        if is_reserved(path) {
            // A reserved file is not a concept; its own structure (§8/§9) is
            // checked once the whole tree is known, so keep its text for then.
            out.reserved.push((file, std::fs::read_to_string(path)?));
        } else if is_markdown(path) {
            let text = std::fs::read_to_string(path)?;
            match Concept::parse(&text) {
                Ok(concept) => {
                    out.findings.extend(check_concept(&file, &concept));
                    out.add_concept(strip_md(&file), file, concept);
                }
                Err(err) => out.findings.push(Finding::new(
                    file,
                    Rule::NotAConcept,
                    format!("not a concept document: {err}"),
                )),
            }
        }
    }
    Ok(())
}

/// The per-concept (document-level) findings for one concept, located at its
/// file. Each check is the spec's own MUST/REQUIRED: a bundle that trips one is
/// non-conformant, not merely unusual.
fn check_concept(file: &str, concept: &Concept) -> Vec<Finding> {
    let fm = concept.frontmatter();
    let mut findings = Vec::new();

    // CONCEPT-2: `type` is required and non-empty (§11).
    if fm.concept_type().is_none_or(|t| t.trim().is_empty()) {
        findings.push(Finding::new(
            file,
            Rule::MissingType,
            "concept declares no non-empty `type` (SPEC §11)",
        ));
    }

    // CONCEPT-3: a declared `status` must be one of the three values (§5.4).
    if fm.declares("status") && fm.status().is_none() {
        findings.push(Finding::new(
            file,
            Rule::InvalidStatus,
            "`status` is not one of draft / stable / deprecated (SPEC §5.4)",
        ));
    }

    // CONCEPT-12: a declared `generated.at` must be an RFC 3339 datetime (§5.2).
    if let Some(at) = fm.generated().and_then(|g| g.at) {
        findings.extend(timestamp_finding(file, "generated.at", &at));
    }

    // CONCEPT-4 / CONCEPT-5: a declared `generated` needs a `by` (§5.2), and it
    // must be a well-formed actor (§7).
    if fm.declares("generated") {
        match fm.generated().and_then(|g| g.by) {
            None => findings.push(Finding::new(
                file,
                Rule::MissingGeneratedBy,
                "`generated` declares no `by` (SPEC §5.2)",
            )),
            Some(by) if by.trim().is_empty() => findings.push(Finding::new(
                file,
                Rule::MissingGeneratedBy,
                "`generated.by` is empty (SPEC §5.2)",
            )),
            Some(by) => findings.extend(actor_finding(file, "generated.by", &by)),
        }
    }

    // CONCEPT-5 / CONCEPT-12: every `verified[]` event must carry a well-formed
    // actor (§7) and an RFC 3339 `at` (§5.2).
    for (i, event) in fm.verified().iter().enumerate() {
        if let Some(by) = event.by.as_deref() {
            findings.extend(actor_finding(file, &format!("verified[{i}].by"), by));
        }
        if let Some(at) = event.at.as_deref() {
            findings.extend(timestamp_finding(file, &format!("verified[{i}].at"), at));
        }
    }

    // CONCEPT-13: a declared `stale_after` must be a `YYYY-MM-DD` date (§5.5).
    if let Some(stale_after) = fm.stale_after() {
        if Date::parse(&stale_after).is_none() {
            findings.push(Finding::new(
                file,
                Rule::MalformedStaleAfter,
                format!("`stale_after` = `{stale_after}` is not a `YYYY-MM-DD` date (SPEC §5.5)"),
            ));
        }
    }

    // CONCEPT-14: the shared `usage_window` frames every `usage_count` (§5.1),
    // so a bound nothing can read costs every source's signal, not one.
    findings.extend(window_findings(file, "usage_window", fm.usage_window()));

    // CONCEPT-6 / CONCEPT-5: each source needs a `resource` (§5.1 REQUIRED), and
    // an `author`, if present, must be a well-formed actor (§7).
    for (i, source) in fm.sources().iter().enumerate() {
        if source
            .resource
            .as_deref()
            .is_none_or(|r| r.trim().is_empty())
        {
            findings.push(Finding::new(
                file,
                Rule::MissingSourceResource,
                format!("`sources[{i}]` declares no `resource` (SPEC §5.1)"),
            ));
        }
        if let Some(author) = source.author.as_deref() {
            findings.extend(actor_finding(file, &format!("sources[{i}].author"), author));
        }

        // CONCEPT-14: the per-source credibility signals (§5.1).
        if let Some(last_modified) = source.last_modified.as_deref() {
            findings.extend(date_finding(
                file,
                &format!("sources[{i}].last_modified"),
                last_modified,
            ));
        }
        if source.usage_count_malformed {
            findings.push(Finding::new(
                file,
                Rule::MalformedSourceSignal,
                format!("`sources[{i}].usage_count` is not an integer (SPEC §5.1)"),
            ));
        }
        findings.extend(window_findings(
            file,
            &format!("sources[{i}].usage_window"),
            source.usage_window.clone(),
        ));
    }

    // §10 Attested Computation — type-conditional: these apply only to a concept
    // whose `type` is exactly `Attested Computation`.
    if fm.concept_type() == Some("Attested Computation") {
        // CONCEPT-7: `runtime` is required for the type (§10.2).
        if fm.runtime().is_none_or(|r| r.trim().is_empty()) {
            findings.push(Finding::new(
                file,
                Rule::MissingRuntime,
                "an Attested Computation declares no `runtime` (SPEC §10.2)",
            ));
        }

        // CONCEPT-8: the computation must come from exactly one place (§10.3) —
        // a `computation:` path or a `# Computation` body block, not both, not
        // neither.
        let has_path = fm.computation().is_some_and(|c| !c.trim().is_empty());
        let has_body = concept.body().has_computation_section();
        if has_path == has_body {
            let detail = if has_path {
                "an Attested Computation gives its computation twice — a `computation:` \
                 path and a `# Computation` body block (SPEC §10.3)"
            } else {
                "an Attested Computation gives its computation nowhere — neither a \
                 `computation:` path nor a `# Computation` body block (SPEC §10.3)"
            };
            findings.push(Finding::new(file, Rule::InvalidComputationSource, detail));
        }

        // CONCEPT-11: a `# Computation` heading that is the sole source but is
        // empty delivers no computation (§10.3) — the same defect as CONCEPT-8's
        // "neither", caught precisely (CONCEPT-8 keys only on heading presence).
        if !has_path && concept.body().has_empty_computation_section() {
            findings.push(Finding::new(
                file,
                Rule::EmptyComputation,
                "the `# Computation` section is empty — the inline computation is \
                 declared but not provided (SPEC §10.3)",
            ));
        }

        // CONCEPT-9: each declared parameter needs name, type, and a boolean
        // `required` (§10.2). A report — parameters are a supporting field.
        for (i, param) in fm.parameters().iter().enumerate() {
            let mut missing = Vec::new();
            if param.name.is_none() {
                missing.push("name");
            }
            if param.kind.is_none() {
                missing.push("type");
            }
            if param.required.is_none() {
                missing.push("a boolean `required`");
            }
            if !missing.is_empty() {
                findings.push(Finding::new(
                    file,
                    Rule::MalformedParameter,
                    format!(
                        "`parameters[{i}]` is missing {} (SPEC §10.2)",
                        missing.join(", ")
                    ),
                ));
            }
        }

        // CONCEPT-10: a declared executor / attester needs a `resource`, and an
        // executor's `receipt` must be a list (§10.2). Reports.
        if let Some(executor) = fm.executor() {
            if executor
                .resource
                .as_deref()
                .is_none_or(|r| r.trim().is_empty())
            {
                findings.push(Finding::new(
                    file,
                    Rule::IncompleteAttestation,
                    "`executor` declares no `resource` (SPEC §10.2)",
                ));
            }
        }
        if fm.executor_receipt_malformed() {
            findings.push(Finding::new(
                file,
                Rule::IncompleteAttestation,
                "`executor.receipt` is not a list of field names (SPEC §10.2)",
            ));
        }
        if let Some(attester) = fm.attester() {
            if attester
                .resource
                .as_deref()
                .is_none_or(|r| r.trim().is_empty())
            {
                findings.push(Finding::new(
                    file,
                    Rule::IncompleteAttestation,
                    "`attester` declares no `resource` (SPEC §10.2)",
                ));
            }
        }
    }

    findings
}

/// A `CONCEPT-5` finding when `value` is present but not a well-formed actor.
///
/// An actor is read permissively — `producer/version` or `scheme:id` — because
/// §7's three-form list excludes its own §5.1 `team:` example. See
/// `docs/okf-friction.md`.
fn actor_finding(file: &str, field: &str, value: &str) -> Option<Finding> {
    if is_actor(value) {
        return None;
    }
    Some(Finding::new(
        file,
        Rule::MalformedActor,
        format!("`{field}` = `{value}` is not a `producer/version` or `scheme:id` actor (SPEC §7)"),
    ))
}

/// The `CONCEPT-14` findings for a `{ from, to }` window, in field order.
fn window_findings(file: &str, field: &str, window: Option<UsageWindow>) -> Vec<Finding> {
    let Some(window) = window else {
        return Vec::new();
    };
    [("from", window.from), ("to", window.to)]
        .into_iter()
        .filter_map(|(bound, value)| {
            date_finding(file, &format!("{field}.{bound}"), value.as_deref()?)
        })
        .collect()
}

/// A `CONCEPT-14` finding when `value` is present but not a `YYYY-MM-DD` date.
fn date_finding(file: &str, field: &str, value: &str) -> Option<Finding> {
    if Date::parse(value).is_some() {
        return None;
    }
    Some(Finding::new(
        file,
        Rule::MalformedSourceSignal,
        format!("`{field}` = `{value}` is not a `YYYY-MM-DD` date (SPEC §5.1)"),
    ))
}

/// A `CONCEPT-12` finding when `value` is present but not an RFC 3339 datetime.
///
/// The finding quotes the value, because the failure it exists for is a
/// timestamp that looks right: `2026-W01-1T00:00:00Z` differs from the calendar
/// form in one character and denotes a date eight days earlier.
fn timestamp_finding(file: &str, field: &str, value: &str) -> Option<Finding> {
    if Timestamp::parse(value).is_some() {
        return None;
    }
    let detail = if value.trim().is_empty() {
        format!("`{field}` is empty (SPEC §5.2)")
    } else {
        format!("`{field}` = `{value}` is not an RFC 3339 datetime (SPEC §5.2)")
    };
    Some(Finding::new(file, Rule::MalformedTimestamp, detail))
}

/// A well-formed actor (§7), read permissively (see [`actor_finding`]): a
/// `producer/version` pair, or a `scheme:id` with a non-empty scheme and id.
fn is_actor(value: &str) -> bool {
    if let Some((producer, version)) = value.split_once('/') {
        return !producer.is_empty() && !version.is_empty();
    }
    if let Some((scheme, id)) = value.split_once(':') {
        return !scheme.is_empty()
            && !id.is_empty()
            && scheme
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
    }
    false
}

/// A `*.md` file.
fn is_markdown(path: &Path) -> bool {
    path.extension().is_some_and(|e| e == "md")
}

/// A reserved filename (`index.md` / `log.md`), at any level (§3.1).
fn is_reserved(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|n| n.to_str()),
        Some("index.md" | "log.md")
    )
}

/// The final path segment of a bundle-relative path — the reserved file's name.
fn reserved_name(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

/// A file's path relative to the bundle root, `/`-joined — the locator a
/// finding is reported against, e.g. `tables/orders.md`.
fn rel_path(root: &Path, path: &Path) -> String {
    let rel = path.strip_prefix(root).unwrap_or(path);
    rel.to_string_lossy()
        .replace(std::path::MAIN_SEPARATOR, "/")
}

/// The Concept ID for a relative path: the trailing `.md` removed (§2).
/// `tables/orders.md` → `tables/orders`.
fn strip_md(file: &str) -> String {
    file.strip_suffix(".md").unwrap_or(file).to_string()
}

/// A `BUNDLE-3` finding when a path-valued field names a bundle file that is
/// not present. Only bundle-path and relative targets are checked; a URL or a
/// `sources` scope descriptor is not a path and is skipped. Empty values are
/// skipped too.
fn check_path(
    from: &str,
    field: &str,
    value: &str,
    allow_scope: bool,
    files: &BTreeSet<String>,
    findings: &mut Vec<Finding>,
) {
    if value.trim().is_empty() {
        return;
    }
    match classify_path(value, allow_scope) {
        PathKind::BundlePath | PathKind::Relative => {
            if !files.contains(&resolve_path(from, value)) {
                findings.push(Finding::new(
                    format!("{from}.md"),
                    Rule::DanglingPath,
                    format!("`{field}` path `{value}` resolves to no file in the bundle"),
                ));
            }
        }
        PathKind::Url | PathKind::ScopeDescriptor => {}
    }
}

/// The Concept ID a body link points at, or `None` when the link cannot name a
/// concept (external, a bare fragment, or a target that is not a `.md` file).
///
/// Shares [`resolve_path`] with the path-valued fields for the `.`/`..`
/// normalisation, then strips the `.md` a concept link must carry.
fn resolve_concept_target(from: &str, link: &Link) -> Option<String> {
    match link.kind {
        LinkKind::BundleAbsolute | LinkKind::Relative => {}
        LinkKind::External | LinkKind::Fragment => return None,
    }
    let path = link.target.split('#').next().unwrap_or("");
    if !path.ends_with(".md") {
        return None;
    }
    Some(strip_md(&resolve_path(from, &link.target)))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A duplicate Concept ID cannot arise from a single filesystem tree, so the
    /// defensive `BUNDLE-1` guard is exercised directly: the first file keeps the
    /// id and the second is reported.
    #[test]
    fn a_duplicate_concept_id_is_reported_and_the_first_file_wins() {
        let mut bundle = Bundle::default();
        let first = Concept::parse("---\ntype: First\n---\n").expect("parses");
        let second = Concept::parse("---\ntype: Second\n---\n").expect("parses");

        bundle.add_concept("dup".into(), "a.md".into(), first);
        bundle.add_concept("dup".into(), "b.md".into(), second);

        assert_eq!(bundle.len(), 1);
        assert_eq!(
            bundle.concept("dup").unwrap().frontmatter().concept_type(),
            Some("First")
        );
        let dups: Vec<_> = bundle
            .findings()
            .iter()
            .filter(|f| f.rule == Rule::DuplicateId)
            .collect();
        assert_eq!(dups.len(), 1);
        assert_eq!(dups[0].file, "b.md");
    }

    fn link(target: &str, kind: LinkKind) -> Link {
        Link {
            target: target.to_string(),
            kind,
        }
    }

    #[test]
    fn resolution_normalises_absolute_and_relative_targets() {
        // absolute ignores the linker; the leading `/`, the `.md`, and a
        // trailing `#fragment` are all stripped.
        assert_eq!(
            resolve_concept_target("anywhere", &link("/a/b.md#x", LinkKind::BundleAbsolute))
                .as_deref(),
            Some("a/b")
        );
        // relative resolves against the linker's directory, applying `..`.
        assert_eq!(
            resolve_concept_target("a/b/c", &link("../sibling.md", LinkKind::Relative)).as_deref(),
            Some("a/sibling")
        );
    }

    #[test]
    fn a_non_concept_target_resolves_to_nothing() {
        assert_eq!(
            resolve_concept_target("a", &link("https://x", LinkKind::External)),
            None
        );
        assert_eq!(
            resolve_concept_target("a", &link("#section", LinkKind::Fragment)),
            None
        );
        // a directory link is not a concept link.
        assert_eq!(
            resolve_concept_target("a", &link("/tables/", LinkKind::BundleAbsolute)),
            None
        );
    }
}
