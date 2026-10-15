//! Structure of a `log.md` reserved file (§9): its date headings must be ISO
//! 8601 `YYYY-MM-DD` (an explicit MUST) and are meant to run newest-first. This
//! module reads only the headings; entry resolution is the bundle's job.

/// What a log's date headings violate (§9): the date headings that are not
/// ISO-8601, and whether the ISO ones run newest-first.
pub(crate) struct HeadingCheck {
    /// The date-heading texts that are not `YYYY-MM-DD`.
    pub non_iso: Vec<String>,
    /// Whether the valid dates are out of newest-first (descending) order.
    pub out_of_order: bool,
}

/// Read a log's date headings, skipping fenced code. ISO dates sort
/// lexicographically, so newest-first is a plain descending check.
///
/// A date heading is any heading — ATX at any level, or setext — whose text
/// holds a digit. §9 says "Date headings MUST use ISO 8601" and never fixes a
/// level; its example uses `##`, and reading only `##` let the same non-ISO
/// date pass at `###` or `#` (#133). The digit test is what tells a date
/// heading from a title such as `# Directory Update Log` or a `## Notes`
/// section, which §9 does not call a date heading and this rule does not
/// judge. A heading with digits and no ISO date is the rule's whole subject:
/// `May 22, 2026` is a date written the way §9 forbids.
pub(crate) fn check_headings(body: &str) -> HeadingCheck {
    let mut non_iso = Vec::new();
    let mut dates: Vec<&str> = Vec::new();
    let mut fence = crate::links::Fence::default();
    // The previous non-blank body line, for a setext underline to attach to.
    let mut previous: Option<&str> = None;
    for raw in body.lines() {
        if fence.step(raw) {
            previous = None;
            continue;
        }
        let line = raw.trim_start();
        let heading = if let Some(text) = atx_heading(line) {
            previous = None;
            Some(text)
        } else if is_setext_underline(line) {
            previous.take()
        } else {
            previous = (!line.trim().is_empty()).then_some(line.trim());
            None
        };
        let Some(heading) = heading else {
            continue;
        };
        if !heading.chars().any(|c| c.is_ascii_digit()) {
            continue; // a title or section name, not a date heading
        }
        if is_iso_date(heading) {
            dates.push(heading);
        } else {
            non_iso.push(heading.to_string());
        }
    }
    HeadingCheck {
        non_iso,
        out_of_order: dates.windows(2).any(|pair| pair[0] < pair[1]),
    }
}

/// The text of an ATX heading (`#` to `######`, then a space), trimmed.
fn atx_heading(line: &str) -> Option<&str> {
    let hashes = line.len() - line.trim_start_matches('#').len();
    if hashes == 0 || hashes > 6 {
        return None;
    }
    let rest = &line[hashes..];
    if !rest.starts_with(' ') && !rest.is_empty() {
        return None;
    }
    Some(rest.trim().trim_end_matches('#').trim())
}

/// A setext underline: a line of only `=` or only `-`, at least one long.
fn is_setext_underline(line: &str) -> bool {
    let line = line.trim();
    !line.is_empty() && (line.chars().all(|c| c == '=') || line.chars().all(|c| c == '-'))
}

/// A `YYYY-MM-DD` date: ten characters, hyphens in place, digits elsewhere, and
/// a plausible month and day. Full calendar validity (no Feb 30) is more than
/// §9 needs.
fn is_iso_date(text: &str) -> bool {
    let bytes = text.as_bytes();
    if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return false;
    }
    let digits = |range: &[u8]| range.iter().all(u8::is_ascii_digit);
    if !digits(&bytes[0..4]) || !digits(&bytes[5..7]) || !digits(&bytes[8..10]) {
        return false;
    }
    let month = text[5..7].parse::<u8>().unwrap_or(0);
    let day = text[8..10].parse::<u8>().unwrap_or(0);
    (1..=12).contains(&month) && (1..=31).contains(&day)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_newest_first_iso_log_is_clean() {
        let body = "# Log\n\n## 2026-05-22\n\n- update\n\n## 2026-05-15\n\n- init\n";
        let check = check_headings(body);
        assert!(check.non_iso.is_empty());
        assert!(!check.out_of_order);
    }

    #[test]
    fn a_non_iso_heading_is_flagged() {
        let check = check_headings("# Log\n\n## May 22, 2026\n\n- update\n");
        assert_eq!(check.non_iso, ["May 22, 2026"]);
        assert!(!check.out_of_order);
    }

    #[test]
    fn ascending_dates_are_out_of_order() {
        let check = check_headings("# Log\n\n## 2026-05-15\n\n## 2026-05-22\n");
        assert!(check.non_iso.is_empty());
        assert!(check.out_of_order);
    }

    /// §9 fixes no heading level, so a date heading is read at any level and
    /// in setext form; a heading with no digit is a title, not a date (#133).
    #[test]
    fn date_headings_are_read_at_any_level_and_titles_are_not_dates() {
        let body = "\
# Directory Update Log

### May 22, 2026

- update

2026-05-15
----------

- init

## Notes

- prose
";
        let check = check_headings(body);
        assert_eq!(check.non_iso, ["May 22, 2026"]);
        assert!(!check.out_of_order);

        // Order is still read across levels.
        let check = check_headings("# 2026-05-15\n\n### 2026-05-22\n");
        assert!(check.non_iso.is_empty());
        assert!(check.out_of_order);
    }

    #[test]
    fn a_date_inside_a_fence_is_not_a_heading() {
        let check = check_headings("# Log\n\n```\n## not-a-date\n```\n");
        assert!(check.non_iso.is_empty());
        assert!(!check.out_of_order);
    }
}
