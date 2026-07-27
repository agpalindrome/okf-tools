//! Structure of an `index.md` reserved file (§8, §12): the frontmatter rule —
//! none, except a bundle-root `okf_version` — and whether a declared
//! `okf_version` is one this tool understands. Entry resolution is the bundle's
//! job (it knows the files an entry might point at); this module reads only the
//! index's own frontmatter.

use serde_yaml::Value;

/// The OKF versions this crate reads: 0.2 (current) and 0.1 (which 0.2
/// supersedes with fallbacks). Any other declared version is `UnknownOkfVersion`
/// — a report, since §12 asks for best-effort consumption, not rejection.
pub(crate) fn understands(version: &str) -> bool {
    matches!(version, "0.1" | "0.2")
}

/// What an index's frontmatter block violates: keys it may not carry here, and
/// a declared `okf_version` this tool does not understand.
pub(crate) struct FrontmatterCheck {
    /// Keys present that are not allowed on this index.
    pub illegal_keys: Vec<String>,
    /// A declared `okf_version` outside {0.1, 0.2}.
    pub unknown_version: Option<String>,
}

/// Check an index's frontmatter block. `is_root` is true only for the
/// bundle-root `index.md`, the one place a block — holding only `okf_version` —
/// is permitted (§8/§12). An empty block declares nothing and is fine; anything
/// present but unparseable is a block it may not carry.
pub(crate) fn check_frontmatter(frontmatter: &str, is_root: bool) -> FrontmatterCheck {
    let mut illegal_keys = Vec::new();
    let mut unknown_version = None;
    match serde_yaml::from_str::<Value>(frontmatter) {
        Ok(Value::Mapping(map)) => {
            for (key, value) in &map {
                let key = key.as_str().unwrap_or("?");
                if is_root && key == "okf_version" {
                    if let Some(version) = value.as_str() {
                        if !understands(version) {
                            unknown_version = Some(version.to_string());
                        }
                    }
                } else {
                    illegal_keys.push(key.to_string());
                }
            }
        }
        Ok(Value::Null) => {}
        Ok(_) | Err(_) => illegal_keys.push("(unparseable frontmatter)".to_string()),
    }
    FrontmatterCheck {
        illegal_keys,
        unknown_version,
    }
}
