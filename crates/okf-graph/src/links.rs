//! Extracting and classifying the markdown links in a concept body (§6.1).
//!
//! Handles **inline** `[text](target)` links (including the `(target "title")`
//! and `(<target>)` forms), skips fenced code blocks, and ignores links inside
//! an inline code span (`` `[x](y)` `` is code, not a link). A code span that
//! runs across lines (rare; fenced blocks are handled separately) is the one
//! residual — masking is per line.
//!
//! Reference-style links (`[text][ref]` + `[ref]: …`) are not handled yet, and
//! are documented rather than dropped silently — a missed link is a missed
//! dangling edge, which is exactly the bug this crate exists to catch (#54).
//!
//! Resolution to a Concept ID is a separate step (it needs the whole bundle);
//! this module only reads a body and says what each link *is*.

/// A markdown link found in a concept body, with its target as written (the
/// `#fragment`, if any, is kept — stripping it belongs to resolution).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Link {
    /// The link target exactly as written.
    pub target: String,
    /// How the target is interpreted.
    pub kind: LinkKind,
}

/// How a link target resolves. Only the first two can point at a concept.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkKind {
    /// `/`-rooted: interpreted relative to the bundle root.
    BundleAbsolute,
    /// A relative path: interpreted against the linking concept's directory.
    Relative,
    /// Has a URL scheme (`https:`) or is protocol-relative (`//host`) — not a
    /// concept link.
    External,
    /// A bare `#fragment`: an intra-document anchor, not a concept link.
    Fragment,
}

/// Every inline link in a body, in document order, skipping fenced code blocks.
pub fn links_in(body: &str) -> Vec<Link> {
    let mut links = Vec::new();
    let mut in_fence = false;
    for line in body.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_fence = !in_fence;
            continue;
        }
        if !in_fence {
            extract_line(line, &mut links);
        }
    }
    links
}

/// Scan one line for `[text](target)` links, pushing each target found. The
/// `](` marker is the anchor; the link text before it is not needed here.
/// Inline code spans are masked first, so a `` `[x](y)` `` inside code is not
/// mistaken for a link.
fn extract_line(line: &str, out: &mut Vec<Link>) {
    let masked = mask_code_spans(line);
    let mut rest = masked.as_str();
    while let Some(pos) = rest.find("](") {
        let after = &rest[pos + 2..];
        match parse_target(after) {
            Some((target, consumed)) => {
                if !target.is_empty() {
                    out.push(Link {
                        kind: classify(&target),
                        target,
                    });
                }
                rest = &after[consumed..];
            }
            None => rest = after,
        }
    }
}

/// Blank out inline code spans — a run of N backticks opens a span the next run
/// of exactly N backticks closes — so a `` `[x](y)` `` inside code is not read
/// as a link. Masked bytes become spaces, preserving every offset so the
/// surrounding text (and any real link in it) is untouched; an unclosed run is
/// left as-is.
fn mask_code_spans(line: &str) -> String {
    let bytes = line.as_bytes();
    let mut out = bytes.to_vec();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'`' {
            i += 1;
            continue;
        }
        let run = bytes[i..].iter().take_while(|&&b| b == b'`').count();
        let mut j = i + run;
        let close = loop {
            if j >= bytes.len() {
                break None;
            }
            if bytes[j] == b'`' {
                let here = bytes[j..].iter().take_while(|&&b| b == b'`').count();
                if here == run {
                    break Some(j);
                }
                j += here;
            } else {
                j += 1;
            }
        };
        match close {
            Some(close) => {
                out[i..close + run].fill(b' ');
                i = close + run;
            }
            None => i += run,
        }
    }
    String::from_utf8(out).unwrap_or_else(|_| line.to_string())
}

/// Read a link target from the text just after `](`, returning the target and
/// how many bytes were consumed up to and including the closing `)`. The
/// `(<target>)` and `(target "title")` forms are both handled; a target with no
/// closing `)` is malformed and yields `None`.
fn parse_target(after: &str) -> Option<(String, usize)> {
    if let Some(inner) = after.strip_prefix('<') {
        let close = inner.find('>')?;
        let paren = inner[close + 1..].find(')')?;
        Some((inner[..close].trim().to_string(), 1 + close + 1 + paren + 1))
    } else {
        let end = after.find(|c: char| c.is_whitespace() || c == ')')?;
        let paren = after[end..].find(')')?;
        Some((after[..end].to_string(), end + paren + 1))
    }
}

/// Classify a target by its leading shape.
fn classify(target: &str) -> LinkKind {
    if target.starts_with('#') {
        LinkKind::Fragment
    } else if target.starts_with("//") || has_scheme(target) {
        LinkKind::External
    } else if target.starts_with('/') {
        LinkKind::BundleAbsolute
    } else {
        LinkKind::Relative
    }
}

/// A `scheme:` prefix (`https:`, `mailto:`) — a scheme is non-empty, starts
/// with a letter, is made of scheme characters, and holds no `/` (so a path
/// like `a/b:c` is not mistaken for one).
fn has_scheme(target: &str) -> bool {
    let Some(colon) = target.find(':') else {
        return false;
    };
    let scheme = &target[..colon];
    !scheme.is_empty()
        && scheme.starts_with(|c: char| c.is_ascii_alphabetic())
        && scheme
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '.' | '-'))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn targets(body: &str) -> Vec<String> {
        links_in(body).into_iter().map(|l| l.target).collect()
    }

    #[test]
    fn extracts_and_classifies_the_link_forms() {
        let body = "\
See the [customers](/tables/customers.md) table, the
[neighbour](./other.md), a [parent](../top.md), the
[home page](https://example.com), [mail](mailto:a@b.com),
a [section](#schema), and a [bare](foo.md).";
        let links = links_in(body);
        let by_target: Vec<(&str, LinkKind)> =
            links.iter().map(|l| (l.target.as_str(), l.kind)).collect();
        assert_eq!(
            by_target,
            [
                ("/tables/customers.md", LinkKind::BundleAbsolute),
                ("./other.md", LinkKind::Relative),
                ("../top.md", LinkKind::Relative),
                ("https://example.com", LinkKind::External),
                ("mailto:a@b.com", LinkKind::External),
                ("#schema", LinkKind::Fragment),
                ("foo.md", LinkKind::Relative),
            ]
        );
    }

    #[test]
    fn a_protocol_relative_target_is_external() {
        assert_eq!(classify("//cdn.example.com/x"), LinkKind::External);
    }

    #[test]
    fn a_fragment_on_a_path_stays_with_the_target() {
        let links = links_in("[x](/a/b.md#schema)");
        assert_eq!(links[0].target, "/a/b.md#schema");
        assert_eq!(links[0].kind, LinkKind::BundleAbsolute);
    }

    #[test]
    fn a_title_and_angle_brackets_are_stripped_from_the_target() {
        assert_eq!(targets("[a](/x.md \"the title\")"), ["/x.md"]);
        assert_eq!(
            targets("[a](</path with spaces.md>)"),
            ["/path with spaces.md"]
        );
    }

    #[test]
    fn links_inside_a_fenced_code_block_are_ignored() {
        let body = "\
before [real](/a.md)
```
not a [link](/nope.md)
```
after [also-real](/b.md)";
        assert_eq!(targets(body), ["/a.md", "/b.md"]);
    }

    #[test]
    fn a_link_inside_an_inline_code_span_is_ignored() {
        // the code-span target is not a link; a real link beside it still is.
        assert_eq!(
            targets("use `[x](/code.md)` but see [real](/a.md)"),
            ["/a.md"]
        );
        // a backticked target alone yields nothing.
        assert!(targets("`[x](/only-code.md)`").is_empty());
        // a double-backtick span is masked too.
        assert_eq!(targets("``[x](/c.md)`` and [r](/b.md)"), ["/b.md"]);
    }

    #[test]
    fn an_empty_target_and_an_unterminated_one_are_skipped() {
        // `[a]()` has an empty target; `[b](/ok.md` never closes its paren.
        assert!(targets("[a]() and [b](/ok.md").is_empty());
        // once closed, the empty one is still skipped and the real one is kept.
        assert_eq!(targets("[a]() and [b](/ok.md)"), ["/ok.md"]);
    }
}
