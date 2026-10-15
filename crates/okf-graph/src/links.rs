//! Extracting and classifying the markdown links in a concept body (§6.1).
//!
//! Handles inline `[text](target)` links (including the `(target "title")` and
//! `(<target>)` forms) and reference-style links — full `[text][ref]`, collapsed
//! `[text][]`, and shortcut `[ref]` — resolved through their `[ref]: target`
//! definitions. Fenced code blocks, indented code blocks and HTML comments are
//! skipped, and inline code spans are masked (`` `[x](y)` `` is code, not a
//! link). A code span that runs across lines (rare; fenced blocks are handled
//! separately) is the one residual, as masking is per line; and link text with
//! nested brackets (`[a [b]](x)`) matches on the first `]`.
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
    let body = mask_html_comments(body);
    let definitions = collect_definitions(&body);
    let mut links = Vec::new();
    let mut fence = Fence::default();
    let mut previous_blank = true;
    let mut in_indented_code = false;
    for line in body.lines() {
        if fence.step(line) {
            previous_blank = false;
            continue;
        }
        // An indented code block (four spaces or a tab) opens only after a
        // blank line, since CommonMark lets it not interrupt a paragraph or a
        // list item's continuation; it runs until a line that is not indented.
        let indented = line.starts_with("    ") || line.starts_with('\t');
        if indented && (previous_blank || in_indented_code) {
            in_indented_code = true;
            continue;
        }
        let blank = line.trim().is_empty();
        if !blank && !indented {
            in_indented_code = false;
        }
        previous_blank = blank;
        if is_definition(line.trim_start()) {
            continue;
        }
        extract_line(line, &definitions, &mut links);
    }
    links
}

/// Fenced-code state across lines, by CommonMark's rule rather than a toggle:
/// a run of three or more backticks or tildes opens a fence, and only a run of
/// the same character at least as long, alone on its line, closes it. A toggle
/// let a three-backtick line inside a four-backtick fence end it early, and a
/// `~~~` fence be closed by backticks, leaking links out of code (#135).
#[derive(Default)]
pub(crate) struct Fence {
    open: Option<(char, usize)>,
}

impl Fence {
    /// Whether a fence is open, before or after any line.
    pub(crate) fn is_open(&self) -> bool {
        self.open.is_some()
    }

    /// Feed one line; `true` when it is a fence line or inside a fence.
    pub(crate) fn step(&mut self, line: &str) -> bool {
        let trimmed = line.trim_start();
        match self.open {
            Some((ch, len)) => {
                let run = trimmed.chars().take_while(|&c| c == ch).count();
                if run >= len && trimmed[run..].trim().is_empty() {
                    self.open = None;
                }
                true
            }
            None => {
                let ch = match trimmed.chars().next() {
                    Some(c @ ('`' | '~')) => c,
                    _ => return false,
                };
                let run = trimmed.chars().take_while(|&c| c == ch).count();
                if run < 3 {
                    return false;
                }
                self.open = Some((ch, run));
                true
            }
        }
    }
}

/// Blank out `<!-- … -->` comments, newlines kept so line numbers hold. A link
/// inside a comment is not rendered and is not a link (#135).
fn mask_html_comments(body: &str) -> String {
    let mut out = String::with_capacity(body.len());
    let mut rest = body;
    while let Some(start) = rest.find("<!--") {
        out.push_str(&rest[..start]);
        let after = &rest[start..];
        let end = after.find("-->").map(|e| e + 3).unwrap_or(after.len());
        out.extend(
            after[..end]
                .chars()
                .map(|c| if c == '\n' { '\n' } else { ' ' }),
        );
        rest = &after[end..];
    }
    out.push_str(rest);
    out
}

/// Decode `%XX` escapes in a link target, as a renderer does before resolving
/// it: `my%20concept.md` names `my concept.md`. A sequence that is not two hex
/// digits, or that does not decode to UTF-8, leaves the target as written.
pub fn decode_percent(target: &str) -> std::borrow::Cow<'_, str> {
    if !target.contains('%') {
        return std::borrow::Cow::Borrowed(target);
    }
    let bytes = target.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex = &target[i + 1..i + 3];
            if let Ok(byte) = u8::from_str_radix(hex, 16) {
                out.push(byte);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    match String::from_utf8(out) {
        Ok(decoded) => std::borrow::Cow::Owned(decoded),
        Err(_) => std::borrow::Cow::Borrowed(target),
    }
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
    let mut fence = Fence::default();
    for line in body.lines() {
        if fence.step(line) {
            continue;
        }
        let trimmed = line.trim_start();
        if let Some((label, target)) = parse_definition(trimmed) {
            definitions.entry(label).or_insert(target);
        }
    }
    definitions
}

/// A `[label]: target` definition — its lowercased label and target (a leading
/// `<…>` unwrapped, a trailing title dropped). `None` when the line is not a
/// definition, or is a footnote definition (`[^id]:`).
///
/// The label ends at the first `]`, which must be followed by `:` at once. A
/// search for `]:` anywhere in the line read `[Orders](/x.md) … [note]: see`
/// as a definition and dropped the link in front of it (#135).
fn parse_definition(line: &str) -> Option<(String, String)> {
    let rest = line.strip_prefix('[')?;
    let close = rest.find(']')?;
    if !rest[close + 1..].starts_with(':') {
        return None;
    }
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

    /// The four parser gaps the 2026-10-15 audit found (#135), each a
    /// CommonMark rule rather than a spec reading.
    #[test]
    fn a_link_before_a_later_bracket_colon_is_still_a_link() {
        let body = "[Orders](/orders.md) is explained by [note]: see below\n";
        assert_eq!(targets(body), ["/orders.md"]);
    }

    #[test]
    fn a_fence_closes_only_on_a_matching_run() {
        let body = "\
````
```
[leaked](/a.md)
````
~~~
```
[leaked](/b.md)
~~~
[real](/c.md)
";
        assert_eq!(targets(body), ["/c.md"]);
    }

    #[test]
    fn html_comments_and_indented_code_hold_no_links() {
        let body = "\
<!-- [hidden](/a.md) -->
<!--
[hidden](/b.md)
-->
text

    [code](/c.md)

- item
    [continuation](/d.md)
[real](/e.md)
";
        assert_eq!(targets(body), ["/d.md", "/e.md"]);
    }

    #[test]
    fn percent_escapes_decode_for_resolution_only() {
        assert_eq!(decode_percent("/my%20concept.md"), "/my concept.md");
        assert_eq!(decode_percent("/plain.md"), "/plain.md");
        assert_eq!(decode_percent("/odd%2.md"), "/odd%2.md");
        assert_eq!(decode_percent("/bad%FF.md"), "/bad%FF.md");
        // the target is kept as written on the link itself
        assert_eq!(targets("[x](/my%20concept.md)"), ["/my%20concept.md"]);
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
