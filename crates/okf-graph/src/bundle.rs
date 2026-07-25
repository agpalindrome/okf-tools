//! A whole OKF Knowledge Bundle loaded into memory: the concepts a directory
//! tree yields, keyed by Concept ID, and the findings the load produced.
//!
//! The walk reuses the shape of `deon`'s `okf.rs::collect` — recurse a
//! directory for `*.md`, read each file — but this crate owns identity: a
//! Concept ID is the file's bundle-relative path with `.md` removed (SPEC §2),
//! and the reserved `index.md` / `log.md` are excluded from the concept set
//! (§3.1); their own structure is validated elsewhere.

use std::collections::BTreeMap;
use std::path::Path;

use crate::{Concept, Finding};

/// An OKF Knowledge Bundle: its concepts by Concept ID, plus the findings the
/// load produced.
#[derive(Debug, Clone, Default)]
pub struct Bundle {
    concepts: BTreeMap<String, Concept>,
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
        let text = std::fs::read_to_string(path)?;
        if let Ok(concept) = Concept::parse(&text) {
            out.concepts.insert(concept_id(root, path), concept);
        }
    }
    Ok(())
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

/// The Concept ID for a file: its path relative to the bundle root, `/`-joined,
/// with the trailing `.md` removed (§2). `tables/orders.md` → `tables/orders`.
fn concept_id(root: &Path, path: &Path) -> String {
    let rel = path.strip_prefix(root).unwrap_or(path);
    let joined = rel.to_string_lossy().replace(std::path::MAIN_SEPARATOR, "/");
    joined.strip_suffix(".md").unwrap_or(&joined).to_string()
}
