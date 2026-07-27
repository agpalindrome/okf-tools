//! Classifying and resolving the path-valued frontmatter fields (§6.2):
//! `resource`, `sources[].resource`, `computation`, `executor.resource`, and
//! `attester.resource`. Each is a URL, a bundle-relative path, or a relative
//! path — and a `sources[].resource` may instead be a scope descriptor (§5.1),
//! free text naming a population rather than a file. Classification is what
//! tells a path a caller should resolve from one it should leave alone.

/// How a path-valued field's value is interpreted (§6.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathKind {
    /// An absolute URL or protocol-relative reference — external, not resolved
    /// against the bundle.
    Url,
    /// A `/`-rooted path, interpreted from the bundle root.
    BundlePath,
    /// A relative path, interpreted against the field's concept's directory.
    Relative,
    /// Free text naming a population or scope, not a path (§5.1) — only a
    /// `sources[].resource` can be one.
    ScopeDescriptor,
}

/// Classify a path-valued field's value. `allow_scope` is true only for
/// `sources[].resource`, the one field §6.2 lets carry a scope descriptor.
///
/// A scope descriptor is detected heuristically — non-URL, non-`/`-rooted, and
/// containing a space — because the spec gives no marker distinguishing it from
/// a path (knowledge-catalog#236, `docs/okf-friction.md`). The heuristic errs
/// toward `Relative`, so the caller never mistakes a scope descriptor for a
/// dangling path, only the reverse.
pub fn classify_path(value: &str, allow_scope: bool) -> PathKind {
    if is_url(value) {
        PathKind::Url
    } else if value.starts_with('/') {
        PathKind::BundlePath
    } else if allow_scope && value.contains(' ') {
        PathKind::ScopeDescriptor
    } else {
        PathKind::Relative
    }
}

/// Resolve a bundle-path or relative `target` to a bundle-relative file path,
/// applying `.` / `..` against `from` (the Concept ID of the concept that
/// carries the field). A `/`-rooted target resolves from the root and ignores
/// `from`; a `#fragment` is dropped; `..` past the root is clamped.
///
/// Only meaningful for [`PathKind::BundlePath`] / [`PathKind::Relative`]; the
/// caller does not resolve a URL or a scope descriptor. Unlike a concept link,
/// the `.md` suffix is kept, since a path-valued field names a file (which may
/// be a `.py` or `.sql`, not a concept).
pub fn resolve_path(from: &str, target: &str) -> String {
    let mut segments: Vec<&str> = if target.starts_with('/') {
        Vec::new()
    } else {
        let mut dir: Vec<&str> = from.split('/').collect();
        dir.pop(); // the concept's own name, not part of its directory
        dir
    };
    let path = target.split('#').next().unwrap_or("");
    let path = path.strip_prefix('/').unwrap_or(path);
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                segments.pop();
            }
            name => segments.push(name),
        }
    }
    segments.join("/")
}

/// A URL: protocol-relative (`//host`) or carrying a scheme (`https:`).
fn is_url(value: &str) -> bool {
    if value.starts_with("//") {
        return true;
    }
    match value.split_once(':') {
        Some((scheme, _)) => {
            !scheme.is_empty()
                && scheme.starts_with(|c: char| c.is_ascii_alphabetic())
                && scheme
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '.' | '-'))
        }
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_the_path_forms() {
        assert_eq!(classify_path("https://example.com/x", true), PathKind::Url);
        assert_eq!(classify_path("//cdn.example.com/x", true), PathKind::Url);
        assert_eq!(
            classify_path("/references/ga4.md", true),
            PathKind::BundlePath
        );
        assert_eq!(
            classify_path("../computations/revenue.md", true),
            PathKind::Relative
        );
        assert_eq!(
            classify_path("references/attesters/revenue.py", true),
            PathKind::Relative
        );
    }

    #[test]
    fn a_spaced_value_is_a_scope_descriptor_only_where_allowed() {
        // sources[].resource may carry one …
        assert_eq!(
            classify_path("all queries in BigQuery project X", true),
            PathKind::ScopeDescriptor
        );
        // … but no other field can, so there it is read as a (odd) relative path.
        assert_eq!(
            classify_path("all queries in BigQuery project X", false),
            PathKind::Relative
        );
    }

    #[test]
    fn resolves_bundle_and_relative_targets_keeping_the_extension() {
        // bundle-absolute ignores the linking concept; the `.md` stays.
        assert_eq!(
            resolve_path("anywhere", "/references/ga4.md"),
            "references/ga4.md"
        );
        // relative resolves against the concept's directory, applying `..`.
        assert_eq!(
            resolve_path("computations/revenue", "../references/run.py"),
            "references/run.py"
        );
        // a non-md extension is kept — a path-valued field names a file.
        assert_eq!(
            resolve_path("metrics/revenue", "revenue.sql"),
            "metrics/revenue.sql"
        );
        // a trailing #fragment is dropped.
        assert_eq!(resolve_path("a/b", "/c/d.md#x"), "c/d.md");
    }
}
