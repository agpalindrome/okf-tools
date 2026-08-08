//! Extracting and classifying the markdown links in a concept body (§6.1).
//!
//! Handles inline `[text](target)` links (including the `(target "title")` and
//! `(<target>)` forms) and reference-style links — full `[text][ref]`, collapsed
//! `[text][]`, and shortcut `[ref]` — resolved through their `[ref]: target`
//! definitions. Fenced code blocks are skipped, and inline code spans are masked
//! (`` `[x](y)` `` is code, not a link). A code span that runs across lines
//! (rare; fenced blocks are handled separately) is the one residual, as masking
//! is per line; and link text with nested brackets (`[a [b]](x)`) matches on the
//! first `]`.
//!
//! Resolution to a Concept ID is a separate step (it needs the whole bundle);
//! this module only reads a body and says what each link *is*.

use std::collections::BTreeMap;

/// A markdown link found in a concept body, with its target as written (the
/// `#fragment`, if any, is kept — stripping it belongs to resolution).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Link {
    /// The link target exactly as written.
    pub target: String,
    /// How the target is interpreted.
    pub kind: LinkKind,
}

/// How a link target resolves. Only the first two can point at a concept.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
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

/// Every link in a body, in document order. Two passes: collect the
/// `[ref]: target` definitions, then scan for links (inline and reference),
/// skipping fenced code blocks and the definition lines themselves.
pub fn links_in(body: &str) -> Vec<Link> {
    let definitions = collect_definitions(body);
    let mut links = Vec::new();
    let mut in_fence = false;
    for line in body.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence || is_definition(trimmed) {
            continue;
        }
        extract_line(line, &definitions, &mut links);
    }
    links
}

/// Scan one line, pushing each link's target: inline `[text](target)`, and
/// reference `[text][ref]` / `[text][]` / `[ref]` resolved through
/// `definitions`. Inline code spans are masked first, so a `` `[x](y)` `` inside
/// code is not mistaken for a link. Link text matches on the first `]`.
fn extract_line(line: &str, definitions: &BTreeMap<String, String>, out: &mut Vec<Link>) {
    let masked = mask_code_spans(line);
    let mut rest = masked.as_str();
    while let Some(open) = rest.find('[') {
        let after_open = &rest[open + 1..];
        let Some(close) = after_open.find(']') else {
            break;
        };
        let text = &after_open[..close];
        let tail = &after_open[close + 1..];

        if let Some(inline) = tail.strip_prefix('(') {
            // inline: [text](target)
            match parse_target(inline) {
                Some((target, consumed)) => {
                    push_target(&target, out);
                    rest = &inline[consumed..];
                }
                None => rest = tail,
            }
        } else if let Some(reference) = tail.strip_prefix('[') {
            // full [text][ref] or collapsed [text][]
            match reference.find(']') {
                Some(ref_close) => {
                    let label = &reference[..ref_close];
                    let key = if label.is_empty() { text } else { label };
                    resolve_reference(key, definitions, out);
                    rest = &reference[ref_close + 1..];
                }
                None => rest = tail,
            }
        } else {
            // shortcut [ref]
            resolve_reference(text, definitions, out);
            rest = tail;
        }
    }
}

/// Push a link for `target` unless it is empty.
fn push_target(target: &str, out: &mut Vec<Link>) {
    if !target.is_empty() {
        out.push(Link {
            kind: classify(target),
            target: target.to_string(),
        });
    }
}

/// Push a link for a reference `label`, if a definition resolves it. An
/// undefined label is plain text, not a link.
fn resolve_reference(label: &str, definitions: &BTreeMap<String, String>, out: &mut Vec<Link>) {
    if let Some(target) = definitions.get(&label.to_lowercase()) {
        push_target(target, out);
    }
}

/// Collect the `[label]: target` reference definitions, keyed by lowercased
/// label (the first definition of a label wins). Fenced code blocks and
/// footnote definitions (`[^id]: …`, §5.1) are skipped.
fn collect_definitions(body: &str) -> BTreeMap<String, String> {
    let mut definitions = BTreeMap::new();
    let mut in_fence = false;
    for line in body.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        if let Some((label, target)) = parse_definition(trimmed) {
            definitions.entry(label).or_insert(target);
        }
    }
    definitions
}

/// A `[label]: target` definition — its lowercased label and target (a leading
/// `<…>` unwrapped, a trailing title dropped). `None` when the line is not a
/// definition, or is a footnote definition (`[^id]:`).
fn parse_definition(line: &str) -> Option<(String, String)> {
    let rest = line.strip_prefix('[')?;
    let close = rest.find("]:")?;
    let label = &rest[..close];
    if label.is_empty() || label.starts_with('^') {
        return None;
    }
    let after = rest[close + 2..].trim();
    let target = after.split_whitespace().next()?;
    let target = target
        .strip_prefix('<')
        .and_then(|t| t.strip_suffix('>'))
        .unwrap_or(target);
    if target.is_empty() {
        return None;
    }
    Some((label.to_lowercase(), target.to_string()))
}

/// Whether a start-trimmed line is a reference definition.
fn is_definition(line: &str) -> bool {
    parse_definition(line).is_some()
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
    fn resolves_the_reference_link_forms() {
        let body = "\
The [full form][ga4], the [ga4][] collapsed, and the [ga4] shortcut.

[ga4]: /tables/ga4.md
";
        // all three forms resolve to the definition's target, and classify it.
        let links = links_in(body);
        assert!(links.iter().all(|l| l.target == "/tables/ga4.md"));
        assert!(links.iter().all(|l| l.kind == LinkKind::BundleAbsolute));
        assert_eq!(links.len(), 3);
    }

    #[test]
    fn an_undefined_reference_and_a_footnote_are_not_links() {
        // `[missing]` has no definition; `[^note]` is a footnote (§5.1), and its
        // definition line is not a link definition.
        let body = "See [missing][nope] and a claim.[^note]\n\n[^note]: the source\n";
        assert!(targets(body).is_empty());
    }

    #[test]
    fn a_definition_line_is_not_scanned_as_a_link() {
        // the def line must not read as a shortcut use of its own label.
        assert!(targets("[o]: /b.md\n").is_empty());
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
