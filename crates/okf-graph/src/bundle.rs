//! A whole OKF Knowledge Bundle loaded into memory: the concepts a directory
//! tree yields, keyed by Concept ID, and the findings the load produced.
//!
//! The walk reuses the shape of `deon`'s `okf.rs::collect` — recurse a
//! directory for `*.md`, read each file — but this crate owns identity: a
//! Concept ID is the file's bundle-relative path with `.md` removed (SPEC §2),
//! and the reserved `index.md` / `log.md` are excluded from the concept set
//! (§3.1); their own structure is validated elsewhere.

use std::collections::btree_map::{BTreeMap, Entry};
use std::path::Path;

use crate::links::{links_in, Link, LinkKind};
use crate::{Concept, Finding, Rule};

/// A resolved body-link edge: the linking concept points at another concept in
/// the same bundle (SPEC §6). A link that resolves to no concept is a dangling
/// `BUNDLE-2` report instead, and never an edge.
#[derive(Debug, Clone, PartialEq, Eq)]
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
    links: Vec<BodyLink>,
    findings: Vec<Finding>,
}

impl Bundle {
    /// Load a bundle from a directory tree.
    ///
    /// Every `*.md` outside the reserved names becomes a concept keyed by its
    /// Concept ID. IO errors (an unreadable directory or file) propagate; a
    /// file that is not a well-formed concept does not — it becomes a finding,
    /// so one malformed document never blocks the rest of the bundle.
    pub fn load(root: &Path) -> std::io::Result<Bundle> {
        let mut bundle = Bundle::default();
        collect(root, root, &mut bundle)?;
        bundle.resolve_links();
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

    /// Every finding the load produced.
    pub fn findings(&self) -> &[Finding] {
        &self.findings
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
}

/// Recurse `path`, adding every non-reserved `*.md` to `out` as a concept.
/// Directory entries are visited in path order so any per-file reporting is
/// reproducible.
fn collect(root: &Path, path: &Path, out: &mut Bundle) -> std::io::Result<()> {
    if path.is_dir() {
        let mut entries: Vec<_> = std::fs::read_dir(path)?
            .collect::<std::io::Result<Vec<_>>>()?
            .into_iter()
            .map(|e| e.path())
            .collect();
        entries.sort();
        for entry in entries {
            collect(root, &entry, out)?;
        }
    } else if is_markdown(path) && !is_reserved(path) {
        let file = rel_path(root, path);
        let text = std::fs::read_to_string(path)?;
        match Concept::parse(&text) {
            Ok(concept) => {
                out.findings.extend(check_concept(&file, &concept));
                let id = strip_md(&file);
                out.add_concept(id, file, concept);
            }
            Err(err) => out.findings.push(Finding::new(
                file,
                Rule::NotAConcept,
                format!("not a concept document: {err}"),
            )),
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

    // CONCEPT-5: every `verified[].by` must be a well-formed actor (§7).
    for (i, event) in fm.verified().iter().enumerate() {
        if let Some(by) = event.by.as_deref() {
            findings.extend(actor_finding(file, &format!("verified[{i}].by"), by));
        }
    }

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

/// The Concept ID a body link points at, or `None` when the link cannot name a
/// concept (external, a bare fragment, or a target that is not a `.md` file).
///
/// A bundle-absolute target resolves from the root; a relative one from the
/// linking concept's directory, applying `.`/`..` (`..` past the root is
/// clamped). The `#fragment` is dropped before resolving.
fn resolve_concept_target(from: &str, link: &Link) -> Option<String> {
    let mut segments: Vec<&str> = match link.kind {
        LinkKind::BundleAbsolute => Vec::new(),
        LinkKind::Relative => {
            let mut dir: Vec<&str> = from.split('/').collect();
            dir.pop(); // the concept's own name, not part of its directory
            dir
        }
        LinkKind::External | LinkKind::Fragment => return None,
    };
    let path = link.target.split('#').next().unwrap_or("");
    let path = path.strip_prefix('/').unwrap_or(path);
    if !path.ends_with(".md") {
        return None;
    }
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                segments.pop();
            }
            name => segments.push(name),
        }
    }
    Some(strip_md(&segments.join("/")))
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
